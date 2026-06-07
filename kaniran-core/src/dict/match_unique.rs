//! Port of `ichiran/dict:match-unique` (`dict-grammar.lisp:689`).
//!
//! Consults [`SUFFIX_UNIQUE_ONLY`] for `suffix_class`:
//!
//! - Not in the table → `None`.
//! - Bare entry → `Some(MatchUniqueResult::Bare)`.
//! - `:desu` entry → a `conjugation` query for matches derived from seq
//!   2755350 (じゃない); `Desu` when at least one match is not a じゃない
//!   conjugation, else `None`.
//! - `:sa` entry → an `entry` query for the matches' seqs filtered by
//!   `root_p`; `Sa(rows)` when non-empty, else `None`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_suffix_unique_only_star_::{
    SuffixUniqueOnly, SUFFIX_UNIQUE_ONLY,
};
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::counters::methods::seq;
use crate::dict::word_info_class::WordInfoSeq;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchUniqueResult {
    Bare,
    Sa(Vec<i32>),
    Desu,
}

pub async fn match_unique(
    ctx: &KaniranContext,
    suffix_class: &str,
    matches: &[KaniWordDispatchEnum],
) -> Result<Option<MatchUniqueResult>, sqlx::Error> {
    let Some((_, kind)) = SUFFIX_UNIQUE_ONLY
        .iter()
        .find(|(k, _)| *k == suffix_class)
    else {
        return Ok(None);
    };
    match kind {
        // dict-grammar.lisp:689-693 (defun match-unique) — bare entry
        // case: `(cond ... (t uniq))` returns the matched keyword
        // itself (truthy).
        SuffixUniqueOnly::Bare => Ok(Some(MatchUniqueResult::Bare)),
        // dict-grammar.lisp:522-530 (pushnew (cons :desu (lambda …)))
        SuffixUniqueOnly::Desu => {
            let seqs = collect_seqs(matches);
            // (and seqs (query …)) — empty seqs short-circuit to nil
            // → (length nil) = 0 → (< 0 (length matches)) = T iff
            // matches non-empty.
            let row_count = if seqs.is_empty() {
                0
            } else {
                let rows: Vec<i32> = sqlx::query_scalar(
                    "SELECT seq FROM conjugation \
                     WHERE seq = ANY($1) AND \"from\" = 2755350",
                )
                .bind(&seqs)
                .fetch_all(&ctx.pool)
                .await?;
                rows.len()
            };
            Ok(if row_count < matches.len() {
                Some(MatchUniqueResult::Desu)
            } else {
                None
            })
        }
        // dict-grammar.lisp:486-490 (pushnew (cons :sa (lambda …)))
        SuffixUniqueOnly::Sa => {
            let seqs = collect_seqs(matches);
            if seqs.is_empty() {
                // (and nil …) → nil → None.
                return Ok(None);
            }
            let rows: Vec<i32> = sqlx::query_scalar(
                "SELECT seq FROM entry WHERE seq = ANY($1) AND root_p",
            )
            .bind(&seqs)
            .fetch_all(&ctx.pool)
            .await?;
            // (and seqs (query …)) — non-empty seqs evaluate the
            // query; empty result is nil (falsy), non-empty is the
            // list of root seqs (truthy).
            Ok(if rows.is_empty() {
                None
            } else {
                Some(MatchUniqueResult::Sa(rows))
            })
        }
    }
}

/// dict-grammar.lisp:488,524 (loop for match in matches if (seq match)
/// collect it) — collects the integer seq when `(seq match)` is
/// truthy, skips when nil. `Multi` panics per the module-doc edge-
/// case note (mirrors Lisp's Postgres 42883 error).
fn collect_seqs(matches: &[KaniWordDispatchEnum]) -> Vec<i32> {
    matches
        .iter()
        .filter_map(|m| match seq(m) {
            Some(WordInfoSeq::Single(s)) => Some(s),
            None => None,
            Some(WordInfoSeq::Multi(_)) => panic!(
                "match_unique: compound-text seq returned WordInfoSeq::Multi; \
                 upstream Lisp errors with Postgres 42883 \
                 'operator does not exist: integer = record' for this input"
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::kani_context::KaniranContext;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::simple_text_class::SimpleText;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env (set DATABASE_URL)")
    }

    async fn fetch_kana_rows_for_seq(ctx: &KaniranContext, seq_val: i32) -> Vec<KanaText> {
        sqlx::query_as::<_, KanaText>("SELECT * FROM kana_text WHERE seq = $1")
            .bind(seq_val)
            .fetch_all(&ctx.pool)
            .await
            .expect("query kana_text")
    }

    fn wrap(rows: Vec<KanaText>) -> Vec<KaniWordDispatchEnum> {
        rows.into_iter().map(KaniWordDispatchEnum::Kana).collect()
    }

    fn synthetic_kana(seq_val: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: seq_val,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    // REPL: (match-unique :ii nil) => :II
    #[tokio::test]
    async fn bare_ii_with_empty_matches_returns_bare() {
        let c = ctx().await;
        let out = match_unique(&c, "ii", &[]).await.unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Bare));
    }

    // REPL: (match-unique :ra nil) => :RA
    #[tokio::test]
    async fn bare_ra_returns_bare() {
        let c = ctx().await;
        let out = match_unique(&c, "ra", &[]).await.unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Bare));
    }

    // REPL: (match-unique :mo nil) => :MO
    #[tokio::test]
    async fn bare_mo_returns_bare() {
        let c = ctx().await;
        let out = match_unique(&c, "mo", &[]).await.unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Bare));
    }

    // REPL: (match-unique :unknown nil) => NIL
    // REPL: (match-unique :foo nil) => NIL
    #[tokio::test]
    async fn unknown_class_returns_none() {
        let c = ctx().await;
        assert_eq!(match_unique(&c, "unknown", &[]).await.unwrap(), None);
        assert_eq!(match_unique(&c, "foo", &[]).await.unwrap(), None);
    }

    // REPL: (match-unique :sa nil) => NIL
    #[tokio::test]
    async fn sa_with_empty_matches_returns_none() {
        let c = ctx().await;
        let out = match_unique(&c, "sa", &[]).await.unwrap();
        assert_eq!(out, None);
    }

    // REPL: matches = kana-text rows for seq=10243330 only (non-root).
    //       (match-unique :sa matches) => NIL
    #[tokio::test]
    async fn sa_with_only_non_root_seqs_returns_none() {
        let c = ctx().await;
        let mats = wrap(fetch_kana_rows_for_seq(&c, 10243330).await);
        assert!(!mats.is_empty(), "REPL precondition: kana_text rows exist for seq=10243330");
        let out = match_unique(&c, "sa", &mats).await.unwrap();
        assert_eq!(out, None);
    }

    // REPL: matches = kana-text rows for seq=10243330 + seq=1586010
    //       (the latter is root-p). (match-unique :sa matches) => (1586010)
    #[tokio::test]
    async fn sa_with_mixed_root_returns_root_seqs() {
        let c = ctx().await;
        let mut mats = fetch_kana_rows_for_seq(&c, 10243330).await;
        mats.extend(fetch_kana_rows_for_seq(&c, 1586010).await);
        let wrapped = wrap(mats);
        let out = match_unique(&c, "sa", &wrapped).await.unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Sa(vec![1586010])));
    }

    // REPL: (match-unique :sa (find-word "はや"))
    //   matches seqs: 1586010 1956580 2638250 10243330
    //     => (1586010 1956580 2638250)
    #[tokio::test]
    async fn sa_with_haya_matches_returns_three_root_seqs() {
        let c = ctx().await;
        let mut mats = Vec::new();
        for s in [1586010, 1956580, 2638250, 10243330] {
            mats.extend(fetch_kana_rows_for_seq(&c, s).await);
        }
        let wrapped = wrap(mats);
        let out = match_unique(&c, "sa", &wrapped).await.unwrap();
        let Some(MatchUniqueResult::Sa(mut rows)) = out else {
            panic!("expected Some(Sa(..)), got {:?}", out);
        };
        rows.sort();
        assert_eq!(rows, vec![1586010, 1956580, 2638250]);
    }

    // REPL: (match-unique :desu nil) => NIL
    #[tokio::test]
    async fn desu_with_empty_matches_returns_none() {
        let c = ctx().await;
        let out = match_unique(&c, "desu", &[]).await.unwrap();
        assert_eq!(out, None);
    }

    // REPL: matches = single kana_text row with seq=10597478 (1 conj row from 2755350)
    //       (match-unique :desu matches) => NIL (1 conj row, 1 match, 1<1 false)
    #[tokio::test]
    async fn desu_with_single_jyanai_derivative_returns_none() {
        let c = ctx().await;
        let single = vec![synthetic_kana(10597478)];
        let out = match_unique(&c, "desu", &single).await.unwrap();
        assert_eq!(out, None);
    }

    // REPL: matches = 2 kana_text rows for seq=10597478 (じゃないです variants)
    //       seqs unique → 1; conj rows from 2755350 → 1; (< 1 2) = T
    //       (match-unique :desu matches) => T
    #[tokio::test]
    async fn desu_with_duplicate_jyanai_seqs_returns_desu() {
        let c = ctx().await;
        let mats = wrap(fetch_kana_rows_for_seq(&c, 10597478).await);
        assert_eq!(
            mats.len(),
            2,
            "REPL precondition: kana_text rows for seq=10597478 = 2"
        );
        let out = match_unique(&c, "desu", &mats).await.unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Desu));
    }

    // REPL: matches = all kana_text rows for seqs 10597478, 10597479, 10597480
    //       len=8; conj rows from 2755350 (3 unique seqs) = 3; (< 3 8) = T
    //       (match-unique :desu matches) => T
    #[tokio::test]
    async fn desu_with_all_jyanai_derived_returns_desu() {
        let c = ctx().await;
        let mut mats = Vec::new();
        for s in [10597478, 10597479, 10597480] {
            mats.extend(fetch_kana_rows_for_seq(&c, s).await);
        }
        let wrapped = wrap(mats);
        assert_eq!(
            wrapped.len(),
            8,
            "REPL precondition: 8 kana_text rows across the three seqs"
        );
        let out = match_unique(&c, "desu", &wrapped).await.unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Desu));
    }

    // REPL: matches = 2 rows for seq=10597478 + 3 rows for seq=1586010 (not じゃない-derived)
    //       conj rows for {10597478, 1586010} from 2755350 = 1; (< 1 5) = T
    //       (match-unique :desu mixed) => T
    #[tokio::test]
    async fn desu_with_mixed_jyanai_and_other_returns_desu() {
        let c = ctx().await;
        let mut mats = fetch_kana_rows_for_seq(&c, 10597478).await;
        mats.extend(fetch_kana_rows_for_seq(&c, 1586010).await);
        let wrapped = wrap(mats);
        assert_eq!(wrapped.len(), 5);
        let out = match_unique(&c, "desu", &wrapped).await.unwrap();
        assert_eq!(out, Some(MatchUniqueResult::Desu));
    }

    // REPL: (make-instance 'compound-text … :words (list kt1 kt2)) → (seq …)
    //       returns the children's seqs as a list; (match-unique :sa (list ct))
    //       errors with Postgres "42883: operator does not exist: integer = record".
    //       The Rust port mirrors this failure mode by panicking — pinning the
    //       behavior so a future caller change doesn't silently substitute
    //       a different shape.
    #[tokio::test]
    #[should_panic(expected = "compound-text seq returned WordInfoSeq::Multi")]
    async fn sa_with_compound_text_match_panics() {
        use crate::dict::compound_text_class::{CompoundText, ScoreMod};
        let c = ctx().await;
        let child1 = synthetic_kana(1586010);
        let child2 = synthetic_kana(10597478);
        let compound = KaniWordDispatchEnum::Compound(CompoundText {
            text: "compound".into(),
            kana: "compound".into(),
            primary: Box::new(child1.clone()),
            words: vec![child1, child2],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        });
        let _ = match_unique(&c, "sa", &[compound]).await;
    }

    // Same panic must fire for the :desu DB branch — both querying paths
    // share `collect_seqs` and must abort identically.
    #[tokio::test]
    #[should_panic(expected = "compound-text seq returned WordInfoSeq::Multi")]
    async fn desu_with_compound_text_match_panics() {
        use crate::dict::compound_text_class::{CompoundText, ScoreMod};
        let c = ctx().await;
        let child1 = synthetic_kana(1586010);
        let child2 = synthetic_kana(10597478);
        let compound = KaniWordDispatchEnum::Compound(CompoundText {
            text: "compound".into(),
            kana: "compound".into(),
            primary: Box::new(child1.clone()),
            words: vec![child1, child2],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        });
        let _ = match_unique(&c, "desu", &[compound]).await;
    }
}
