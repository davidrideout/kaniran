//! Port of `ichiran/dict:find-word-suffix` (`dict-grammar.lisp:695`).
//!
//! Iterates over the suffix triples for `word` (from the precomputed
//! suffix-map-temp when bound, else [`get_suffixes`]), dispatches each
//! through [`SUFFIX_LIST`], and concatenates the resulting rows. Each
//! suffix-fn runs with `suffix_next_end` decremented by the suffix's
//! character length so a nested call can peel further. Offsets are
//! character positions, not bytes.
//!
//! For the match-unique gate, the suffix-class is the value in
//! [`SUFFIX_CLASS`] for `kf.seq` when `kf` is present, otherwise the
//! keyword itself.
//!
//! [`SUFFIX_LIST`]: super::_star_suffix_list_star_::SUFFIX_LIST
//! [`get_suffixes`]: super::get_suffixes::get_suffixes
//! [`SUFFIX_CLASS`]: super::_star_suffix_class_star_::suffix_class

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_suffix_class_star_::suffix_class;
use crate::dict::_star_suffix_list_star_::lookup_suffix_fn;
use crate::dict::get_suffixes::get_suffixes;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::match_unique::match_unique;
use crate::dict::subseq_slice::subseq_slice;

pub async fn find_word_suffix(
    ctx: &KaniranContext,
    word: &str,
    matches: &[KaniWordDispatchEnum],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let word_len = word.chars().count();

    // dict-grammar.lisp:696-698 (with suffixes = (if *suffix-map-temp* …))
    // Two sources keep distinct ownership: a borrowed slice over the
    // ctx-owned map, or an owned Vec from get_suffixes.
    let suffixes_owned: Vec<(String, String, Option<KanaText>)>;
    let suffixes_from_map: Option<&[(String, String, Option<KanaText>)]>;

    if let Some(map) = ctx.suffix_map_temp.as_deref() {
        // dict-grammar.lisp:697 (gethash *suffix-next-end* *suffix-map-temp*)
        // Negative *suffix-next-end* values match no hash key (gethash
        // returns nil); SuffixMapTemp keys are usize so we mirror via
        // `try_from`.
        let key = ctx
            .suffix_next_end
            .and_then(|e| usize::try_from(e).ok());
        suffixes_from_map = key.and_then(|k| map.get(&k)).map(|v| v.as_slice());
        suffixes_owned = Vec::new();
    } else {
        // dict-grammar.lisp:698 (get-suffixes word)
        // get_suffixes returns borrowed slices into ctx; convert to
        // owned triples so the loop body can be uniform.
        suffixes_owned = get_suffixes(ctx, word)
            .into_iter()
            .map(|(s, k, kf)| (s.to_string(), k.to_string(), kf.cloned()))
            .collect();
        suffixes_from_map = None;
    }
    let suffix_triples: &[(String, String, Option<KanaText>)] = match suffixes_from_map {
        Some(s) => s,
        None => suffixes_owned.as_slice(),
    };

    let class_map = suffix_class(ctx);
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();

    // dict-grammar.lisp:700 (for (suffix keyword kf) in suffixes)
    for (suffix, keyword, kf) in suffix_triples {
        // dict-grammar.lisp:701 (cdr (assoc keyword *suffix-list*))
        let Some(suffix_fn) = lookup_suffix_fn(keyword) else {
            continue;
        };
        let suffix_len = suffix.chars().count();
        // dict-grammar.lisp:703 (- (length word) (length suffix))
        // Use checked_sub to mirror Lisp's signed arithmetic — Lisp
        // would produce a negative offset for over-long suffixes,
        // failing the (> offset 0) gate; in Rust we treat the
        // saturating-to-zero case the same way.
        let Some(offset) = word_len.checked_sub(suffix_len) else {
            continue;
        };
        // dict-grammar.lisp:704 (and suffix-fn (> offset 0) ...)
        if offset == 0 {
            continue;
        }
        // dict-grammar.lisp:702 (if kf (gethash (seq kf) *suffix-class*) keyword)
        let suffix_class_str: &str = match kf {
            Some(k) => class_map.get(&k.seq).map(String::as_str).unwrap_or(keyword),
            None => keyword,
        };
        // dict-grammar.lisp:705 (not (and matches (match-unique suffix-class matches)))
        if !matches.is_empty()
            && match_unique(ctx, suffix_class_str, matches).await?.is_some()
        {
            continue;
        }
        // dict-grammar.lisp:706 (let ((*suffix-next-end* (and *suffix-next-end* (- *suffix-next-end* (length suffix)))))
        // Lisp `(and nil x)` → nil so the rebind only computes a new
        // value when the current binding is non-nil. Mirror with
        // `Option::map`.
        let new_next_end = ctx.suffix_next_end.map(|e| e - suffix_len as i32);
        let ctx2 = ctx.with_suffix_next_end(new_next_end);
        // dict-grammar.lisp:707 (funcall suffix-fn (subseq-slice slice word 0 offset) suffix kf)
        // subseq_slice's first arg is the legacy `make-slice` seed and
        // is ignored in Rust; pass None per the port note.
        let root = subseq_slice(None, word, 0, Some(offset));
        let compounds = suffix_fn(&ctx2, root, suffix, kf.as_ref()).await?;
        out.extend(compounds);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(find-word-suffix "勉強する")` upstream returns 1
    /// compound via the SURU branch (TEIRU also reaches "る" but
    /// suffix-teiru on root="勉強す" fails its te-check).
    #[tokio::test]
    async fn t1_benkyou_suru() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "勉強する", &[]).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected Compound, got {:?}", r[0]);
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// REPL: `(find-word-suffix "区別し")` → 1 compound (SURU branch
    /// only — the partial cache holds an entry for "し" under :SURU
    /// keyword).
    #[tokio::test]
    async fn t2_kubetsu_shi() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "区別し", &[]).await.unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-suffix "私ら")` → 13 compounds via the RA
    /// branch.
    #[tokio::test]
    async fn t3_watashi_ra_polysemy() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "私ら", &[]).await.unwrap();
        assert_eq!(r.len(), 13);
        for w in &r {
            let KaniWordDispatchEnum::Compound(c) = w else {
                panic!("expected Compound, got {:?}", w);
            };
            assert_eq!(c.text, "私ら");
        }
    }

    /// REPL: `(find-word-suffix "食べてる")` upstream returns 1 via
    /// the TEIRU branch (suffix-teiru's te-check passes on root
    /// "食べて"). The dispatch table now wires `teiru`, so we mirror
    /// upstream and pin the 1-compound outcome.
    #[tokio::test]
    async fn t4_teiru_fires() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "食べてる", &[]).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected Compound, got {:?}", r[0]);
        };
        assert_eq!(c.text, "食べてる");
    }

    /// REPL: `(find-word-suffix "ら")` → NIL. Word length equals
    /// suffix length → offset = 0 → `(> offset 0)` fails → no
    /// expansion.
    #[tokio::test]
    async fn t5_offset_zero_skipped() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "ら", &[]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-suffix "")` → NIL. get-suffixes("") = NIL
    /// (loop range empty), so the iteration body doesn't run.
    #[tokio::test]
    async fn t6_empty_word() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "", &[]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-suffix "私ら" :matches (find-word "私"))` →
    /// NIL. `match-unique` :ra returns :RA (truthy) for the find-word
    /// 私 matches → the row is filtered out and no compounds emit.
    #[tokio::test]
    async fn t7_match_unique_gate_fires() {
        let ctx = ctx().await;
        // Build matches = find-word 私 (kana + kanji rows).
        let watashi_rows = crate::dict::find_word::find_word(&ctx, "私", false)
            .await
            .unwrap();
        let matches: Vec<KaniWordDispatchEnum> = match watashi_rows {
            crate::dict::find_word::FindWordRows::Kana(v) => v
                .into_iter()
                .map(KaniWordDispatchEnum::Kana)
                .collect(),
            crate::dict::find_word::FindWordRows::Kanji(v) => v
                .into_iter()
                .map(KaniWordDispatchEnum::Kanji)
                .collect(),
        };
        assert!(!matches.is_empty(), "REPL precondition: 私 rows exist");
        let r = find_word_suffix(&ctx, "私ら", &matches).await.unwrap();
        assert!(r.is_empty());
    }

    /// Map-path branch coverage (`dict-grammar.lisp:697` — the
    /// `*suffix-map-temp*` source, `find_word_suffix.rs:95-103`). Every
    /// other test here runs with `suffix_map_temp = None` and exercises
    /// only the `get_suffixes` fallback; this one binds a real suffix
    /// map (mirroring `join_substring_words_star_`) so the suffix triples
    /// come from `map[suffix_next_end]`, independent of `word`.
    ///
    /// Sentence "しきれなくなったらしく" — なくなったら ends at char 9.
    /// REPL-verified on the ichiran host: map@9 = (ら たら ったら なったら)
    /// → `find-word-suffix("なくなったら")` = 3; map@8 = (た った なった)
    /// → 0. The next-end=8 case is the nested-call shape (a parent suffix
    /// decremented the end): the map is indexed one position short,
    /// yields the wrong suffix row, and returns 0 where the bare
    /// `get_suffixes` path would have returned 3.
    #[tokio::test]
    async fn t8_map_path_position_sensitive() {
        use crate::dict::_star_suffix_map_temp_star_::SuffixMapTemp;
        use crate::dict::get_suffix_map::get_suffix_map;
        use std::sync::Arc;

        let ctx = ctx().await;
        let sentence = "しきれなくなったらしく";
        // Mirror join_substring_words_star_:72-83 — *suffix-map-temp*
        // owns its triples, so materialize owned copies of the borrowed
        // get_suffix_map output.
        let suffix_map: Arc<SuffixMapTemp> = Arc::new(
            get_suffix_map(&ctx, sentence)
                .into_iter()
                .map(|(end, items)| {
                    let owned: Vec<(String, String, Option<_>)> = items
                        .into_iter()
                        .map(|(s, k, kf)| (s.to_string(), k.to_string(), kf.cloned()))
                        .collect();
                    (end, owned)
                })
                .collect(),
        );

        // map@9 = (ら たら ったら なったら) → 3 compounds.
        let ctx9 = ctx
            .with_suffix_map_temp(Some(Arc::clone(&suffix_map)))
            .with_suffix_next_end(Some(9));
        let r9 = find_word_suffix(&ctx9, "なくなったら", &[]).await.unwrap();
        assert_eq!(r9.len(), 3, "map@9 (ら/たら/ったら/なったら) → 3 compounds");

        // map@8 = (た った なった) — the decremented-end nested-call
        // shape; the suffixes don't align with なくなったら → 0.
        let ctx8 = ctx
            .with_suffix_map_temp(Some(Arc::clone(&suffix_map)))
            .with_suffix_next_end(Some(8));
        let r8 = find_word_suffix(&ctx8, "なくなったら", &[]).await.unwrap();
        assert!(r8.is_empty(), "map@8 (た/った/なった) → no compounds");
    }
}
