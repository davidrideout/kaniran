//! Port of `ichiran/dict:gen-score` (`dict.lisp:985`).
//!
//! ```lisp
//! (defun gen-score (segment &key final kanji-break)
//!   (setf (values (segment-score segment) (segment-info segment))
//!         (calc-score (segment-word segment) :final final :kanji-break kanji-break))
//!   segment)
//! ```
//!
//! Mutates `segment.score` and `segment.info` in place with the
//! `(score, info)` pair returned by [`calc_score`], then returns the
//! same `&mut Segment` so call sites can chain (`segs.push(gen_score(…)…)`).
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injection** per CONVENTIONS §4.8. Lisp lambda list is
//!   `(segment &key final kanji-break)`; the Rust signature prepends
//!   `ctx: &KaniranContext` because [`calc_score`] is ctx-injected.
//!   `audit-signatures` reports the Rust arity as `4 ≠ Lisp 3 …
//!   (ctx-injected; +1 absorbed)`.
//! - **`async fn` + `sqlx::Error` Result** because [`calc_score`] is
//!   async and propagates DB errors.
//! - **`&key` → positional** (CONVENTIONS §4.4). The two upstream
//!   keywords are passed positionally:
//!   - `final` → `final_: bool` (rename only; `final` is a Rust
//!     keyword).
//!   - `kanji-break` → `kanji_break: &[usize]` (empty slice ≡ upstream
//!     `nil`).
//! - **In-place mutation, returns `&mut Segment`** instead of a fresh
//!   value. Upstream uses CLOS `setf` to mutate the segment slots and
//!   returns the same object; the Rust port mirrors that exactly.

use crate::conn::kani_context::KaniranContext;
use crate::dict::calc_score::calc_score;
use crate::dict::segment_struct::Segment;

pub async fn gen_score<'a>(
    ctx: &KaniranContext,
    segment: &'a mut Segment,
    final_: bool,
    kanji_break: &[usize],
) -> Result<&'a mut Segment, sqlx::Error> {
    // dict.lisp:986-987 — (setf (values (segment-score segment) (segment-info segment))
    //                       (calc-score (segment-word segment) :final final :kanji-break kanji-break))
    let (score, info) = calc_score(
        ctx,
        &segment.word,
        final_,
        /* use_length */ None,
        /* score_mod */ None,
        kanji_break,
    )
    .await?;
    segment.score = Some(score);
    segment.info = info;
    // dict.lisp:988 — segment (the function returns the same segment).
    Ok(segment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::{KaniSplitInfo, Segment};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_kana_for(ctx: &KaniranContext, s: &str) -> KaniWordDispatchEnum {
        match find_word(ctx, s, false).await.unwrap() {
            FindWordRows::Kana(mut v) => KaniWordDispatchEnum::Kana(v.remove(0)),
            FindWordRows::Kanji(mut v) => KaniWordDispatchEnum::Kanji(v.remove(0)),
        }
    }

    /// Deterministic single-row fetch — `find-word`'s SQL has no
    /// ORDER BY, so the same lookup can rotate first rows between
    /// runs / databases.
    async fn kana_by_seq_text(
        ctx: &KaniranContext,
        seq: i32,
        text: &str,
    ) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::kana_text_dao::KanaText> = sqlx::query_as(
            "SELECT * FROM kana_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
        )
        .bind(seq)
        .bind(text)
        .fetch_all(&ctx.pool)
        .await
        .expect("query");
        KaniWordDispatchEnum::Kana(rows.into_iter().next().expect("row exists"))
    }

    async fn kanji_by_seq_text(
        ctx: &KaniranContext,
        seq: i32,
        text: &str,
    ) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::kanji_text_dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
        )
        .bind(seq)
        .bind(text)
        .fetch_all(&ctx.pool)
        .await
        .expect("query");
        KaniWordDispatchEnum::Kanji(rows.into_iter().next().expect("row exists"))
    }

    fn make_segment(word: KaniWordDispatchEnum, end: usize, text: &str) -> Segment {
        Segment {
            start: 0,
            end,
            word,
            score: None,
            info: None,
            top: None,
            text: Some(text.to_string()),
        }
    }

    // ----- REPL-pinned cases (.103, 2026-05-16). Captured by running
    //       `(gen-score (make-segment :start 0 :end <n> :word w :text "<txt>"))`
    //       followed by `(segment-score s) / (segment-info s)`. -----

    /// REPL: GEN-SCORE 'ねこ': score=16
    /// info=(:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///       :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))
    #[tokio::test]
    async fn neko_baseline_writes_score_and_info() {
        let ctx = ctx_from_env().await;
        let w = first_kana_for(&ctx, "ねこ").await;
        let mut seg = make_segment(w, 2, "ねこ");
        gen_score(&ctx, &mut seg, false, &[]).await.unwrap();
        assert_eq!(seg.score, Some(16));
        let info = seg.info.as_ref().unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1467640]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(7));
        assert_eq!(info.score_info.prop_score, 4);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, true, false));
    }

    /// REPL: with row `(select-dao 'kanji-text (:and (:= 'seq 2698030) (:= 'text "猫")))` →
    ///   `(gen-score (make-segment :start 0 :end 1 :word w :text "猫") :kanji-break '(0))` →
    ///   score=3, info=(:POSI NIL :SEQ-SET (2698030) :CONJ NIL :COMMON NIL
    ///                  :SCORE-INFO (3 (0) 0 NIL) :KPCL (T NIL NIL NIL))
    ///
    /// Deterministic-row helper avoids `find-word`'s no-ORDER-BY
    /// nondeterminism.
    #[tokio::test]
    async fn neko_kanji_break_propagates_through_calc_score() {
        let ctx = ctx_from_env().await;
        let w = kanji_by_seq_text(&ctx, 2698030, "猫").await;
        let mut seg = make_segment(w, 1, "猫");
        gen_score(&ctx, &mut seg, false, &[0]).await.unwrap();
        assert_eq!(seg.score, Some(3));
        let info = seg.info.as_ref().unwrap();
        assert!(info.posi.is_empty());
        assert_eq!(info.seq_set, vec![2698030]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 3);
        assert_eq!(info.score_info.kanji_break, vec![0]);
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, false, false, false));
    }

    /// REPL: with row `(select-dao 'kana-text (:and (:= 'seq 1290020) (:= 'text "ね")))` →
    ///   `(gen-score (make-segment :start 0 :end 1 :word w :text "ね") :final t)` →
    ///   score=4, info=(:POSI ("n") :SEQ-SET (1290020) :CONJ NIL :COMMON 5
    ///                  :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))
    #[tokio::test]
    async fn ne_final_common_n_branch() {
        let ctx = ctx_from_env().await;
        let w = kana_by_seq_text(&ctx, 1290020, "ね").await;
        let mut seg = make_segment(w, 1, "ね");
        gen_score(&ctx, &mut seg, true, &[]).await.unwrap();
        assert_eq!(seg.score, Some(4));
        let info = seg.info.as_ref().unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1290020]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(5));
        assert_eq!(info.score_info.prop_score, 4);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, true, false));
    }
}
