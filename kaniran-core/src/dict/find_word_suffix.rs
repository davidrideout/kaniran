//! Port of `ichiran/dict:find-word-suffix` (`dict-grammar.lisp:695`).
//!
//! ```lisp
//! (defun find-word-suffix (word &key matches)
//!   (loop with suffixes = (if *suffix-map-temp*
//!                             (gethash *suffix-next-end* *suffix-map-temp*)
//!                             (get-suffixes word))
//!        and slice = (make-slice)
//!      for (suffix keyword kf) in suffixes
//!      for suffix-fn = (cdr (assoc keyword *suffix-list*))
//!      for suffix-class = (if kf (gethash (seq kf) *suffix-class*) keyword)
//!      for offset = (- (length word) (length suffix))
//!      when (and suffix-fn (> offset 0)
//!                (not (and matches (match-unique suffix-class matches))))
//!      nconc (let ((*suffix-next-end* (and *suffix-next-end* (- *suffix-next-end* (length suffix)))))
//!              (funcall suffix-fn (subseq-slice slice word 0 offset) suffix kf))))
//! ```
//!
//! Iterates over the suffix triples for `word`, dispatches each
//! through [`SUFFIX_LIST`], and concatenates the resulting compound-
//! text rows. Two suffix sources:
//!
//! - When `ctx.suffix_map_temp` is bound, look up the precomputed
//!   triples at `ctx.suffix_next_end` in that map.
//! - Otherwise call [`get_suffixes`] to enumerate them on the fly.
//!
//! Each call to `suffix-fn` runs under a sibling ctx with
//! `suffix_next_end` decremented by the suffix's character length —
//! that lets a nested `find-word-suffix` invocation peel further with
//! the same suffix-map-temp data (CONVENTIONS §4.10).
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injected** per CONVENTIONS §4.8 / §4.10. The dynamic
//!   `*suffix-map-temp*` and `*suffix-next-end*` rebinds become
//!   sibling ctx construction via
//!   [`KaniranContext::with_suffix_next_end`].
//! - **`matches: &[KaniWordDispatchEnum]`** instead of a `&key`
//!   keyword. Empty slice mirrors the upstream `nil` default (no
//!   match-unique gate).
//! - **Return is `Vec<CompoundText>`.** Every reachable suffix-fn
//!   produces compounds via [`adjoin_word`]; the find-word-full
//!   caller wraps them in [`KaniWordDispatchEnum::Compound`].
//! - **Character-length offsets** per CONVENTIONS §4.5 — the
//!   `*suffix-next-end*` rebind arithmetic uses character positions,
//!   not bytes.
//! - **Partial `*suffix-list*` table.** Only `suru` and `ra` are
//!   registered today; rows for other keywords from
//!   [`super::get_suffixes`] / [`super::get_suffix_map`] silently
//!   skip via `lookup_suffix_fn` returning `None`. Matches the
//!   upstream `(when suffix-fn …)` gate when a defsuffix body is not
//!   yet loaded.
//!
//! ## suffix-class for the match-unique gate
//!
//! When `kf` is present, the suffix-class is the value carried in
//! [`SUFFIX_CLASS`] for `kf.seq`. When `kf` is absent (`(load-abbr
//! …)` cache rows), upstream falls back to the keyword itself —
//! mirrored by using the keyword string in that branch.
//!
//! ## Empty word
//!
//! `(length word)` = 0 makes `(- 0 (length suffix))` negative for any
//! candidate suffix and the `(> offset 0)` guard drops them all. The
//! [`get_suffixes`] / suffix-map-temp pre-fetch loops never visit a
//! length-zero input (their `start` ranges are empty), so the outer
//! iteration body is unreachable in practice.
//!
//! [`SUFFIX_LIST`]: super::_star_suffix_list_star_::SUFFIX_LIST
//! [`KaniranContext::with_suffix_next_end`]: crate::conn::kani_context::KaniranContext::with_suffix_next_end
//! [`adjoin_word`]: super::adjoin_word::adjoin_word
//! [`get_suffixes`]: super::get_suffixes::get_suffixes
//! [`SUFFIX_CLASS`]: super::_star_suffix_class_star_::suffix_class

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_suffix_class_star_::suffix_class;
use crate::dict::_star_suffix_list_star_::lookup_suffix_fn;
use crate::dict::compound_text_class::CompoundText;
use crate::dict::get_suffixes::get_suffixes;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::match_unique::match_unique;
use crate::dict::subseq_slice::subseq_slice;

pub async fn find_word_suffix(
    ctx: &KaniranContext,
    word: &str,
    matches: &[KaniWordDispatchEnum],
) -> Result<Vec<CompoundText>, sqlx::Error> {
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
    let mut out: Vec<CompoundText> = Vec::new();

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
    /// compound via the SURU branch (TEIRU keyword also matches "る"
    /// at the deepest start but suffix-teiru on root="勉強す" fails
    /// te-check). The port's `*suffix-list*` registers `suru`, so we
    /// see 1 compound here too. (`teiru` is absent but would have
    /// produced 0 anyway.)
    #[tokio::test]
    async fn t1_benkyou_suru() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "勉強する", &[]).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].text, "勉強する");
        assert_eq!(r[0].kana, "べんきょう する");
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
        for c in &r {
            assert_eq!(c.text, "私ら");
        }
    }

    /// REPL: `(find-word-suffix "食べてる")` upstream returns 1 (TEIRU
    /// fires); port's `*suffix-list*` does NOT register `teiru`, so
    /// the row is silently dropped and we observe 0. Pin the
    /// partial-table behavior.
    #[tokio::test]
    async fn t4_unregistered_keyword_drops_silently() {
        let ctx = ctx().await;
        let r = find_word_suffix(&ctx, "食べてる", &[]).await.unwrap();
        assert!(r.is_empty());
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
}
