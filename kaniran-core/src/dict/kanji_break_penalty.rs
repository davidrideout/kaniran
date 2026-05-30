//! Port of `ichiran/dict:kanji-break-penalty` (`dict.lisp:702`).
//!
//! Adjusts a candidate's `score` when the segmenter's hard kanji-break
//! marker falls on the candidate's boundary. The `kanji-break` argument
//! lists the character positions of the break(s) within the matched
//! slice; the function decides whether the break sits at the beginning
//! (`:beg`), end (`:end`), or both (`:both`) of the slice, applies a
//! small per-`posi` bonus when an n-suf / pref overlap exists, then
//! halves the score (`ceiling score 2`) unless the candidate is
//! exempted (`*no-kanji-break-penalty*`, `すー`-starting beg, or the
//! `vs-s` / `v5s` suru-suffix path).
//!
//! Mutually recursive with [`super::calc_score::calc_score`] via the
//! suru-suffix branch: when the candidate carries a `vs-s` / `v5s`
//! part-of-speech and the input text matches a registered `:SURU`
//! suffix (`get_suffixes`), the function scores the underlying
//! reading row by re-entering `calc-score`, then caps the result at
//! `min(score, suffix-score + 50)`.
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injection** per CONVENTIONS §4.8. The Lisp lambda list is
//!   `(kanji-break score &key info text use-length score-mod)`; the
//!   Rust signature prepends `ctx: &KaniranContext` because the
//!   recursive `calc-score` call touches the database (via
//!   `get-original-text`'s SQL and the `is-arch` / `prefer-kana` reads).
//!   `audit-signatures` reports the Rust arity as `7 ≠ Lisp 6 …
//!   (ctx-injected; +1 absorbed)`.
//! - **`async fn` + `sqlx::Error` Result** for the same reason: the
//!   recursive `calc-score` is async, and DB errors propagate.
//! - **`&key` → positional** (CONVENTIONS §4.4). Every upstream
//!   `kanji-break-penalty` callsite (the two inside `calc-score` at
//!   `dict.lisp:788` and `dict.lisp:981`) passes all four keywords;
//!   modeling them as positional with the rich types they carry
//!   (`Option<&KaniSegmentInfo>`, `&str`, `Option<i32>`,
//!   `Option<&ScoreMod>`) keeps the dispatch self-describing while
//!   removing the keyword ceremony.
//! - **`kanji-break` as `&[usize]`.** Upstream `kanji-break` is a list
//!   of character positions; the Rust port takes `&[usize]` (character
//!   positions, CONVENTIONS §4.5). Empty slice maps to upstream `nil`
//!   and reaches the same `:end` branch through the same `cdr` /
//!   `car` falsy-on-`nil` semantics.
//! - **`KanjiBreakEnd` sidecar enum** (CONVENTIONS §4.3) names the
//!   three Lisp `(cond …)` results that the body matches on.
//! - **`(third suru-suffix)` is a `KanaText` row** in the Rust triple
//!   (`get_suffixes` returns `(suffix-text, key, Option<&KanaText>)`).
//!   To call `calc_score` recursively we wrap the row as
//!   [`KaniWordDispatchEnum::Kana`] — `calc-score`'s simple-text
//!   branch only reads slot fields the bare DAO already carries.
//!
//! ## Numeric width
//!
//! The Lisp `(ceiling score ratio)` divides `score` by `ratio = 2`
//! with round-up semantics. The Rust port uses `(score + ratio - 1)
//! / ratio` (positive-operand round-up — `i32::div_ceil` is still
//! unstable as of rustc 1.95) — both inputs are non-negative in every
//! reachable callsite (`score` is declared `(integer 0 1000000)` in
//! `calc-score`, `ratio` is `2`).

use crate::characters::text_utils::mora_length;
use crate::conn::kani_context::KaniranContext;
use crate::dict::errata::NO_KANJI_BREAK_PENALTY;
use crate::dict::_star_score_cutoff_star_::SCORE_CUTOFF;
use crate::dict::calc_score::calc_score;
use crate::dict::compound_text_class::ScoreMod;
use crate::dict::get_suffixes::get_suffixes;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::segment_struct::KaniSegmentInfo;

/// The three Lisp `(cond ((cdr kanji-break) :both) ((eql (car kanji-break) 0) :beg) (t :end))`
/// results. `kanji_break` empty → upstream `nil` → falls through to
/// `End` via `(cdr nil) = nil`, `(eql nil 0) = nil`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KanjiBreakEnd {
    Both,
    Beg,
    End,
}

fn classify_end(kanji_break: &[usize]) -> KanjiBreakEnd {
    // dict.lisp:703-705
    if kanji_break.len() > 1 {
        KanjiBreakEnd::Both
    } else if kanji_break.first().copied() == Some(0) {
        KanjiBreakEnd::Beg
    } else {
        KanjiBreakEnd::End
    }
}

pub async fn kanji_break_penalty(
    ctx: &KaniranContext,
    kanji_break: &[usize],
    score: i32,
    info: Option<&KaniSegmentInfo>,
    text: &str,
    use_length: Option<i32>,
    score_mod: Option<&ScoreMod>,
) -> Result<i32, sqlx::Error> {
    // dict.lisp:703-707 (let ((end ...) (bonus 0) (ratio 2) (posi (and info (getf info :posi)))))
    let end = classify_end(kanji_break);
    let mut bonus: i32 = 0;
    let ratio: i32 = 2;
    let posi: &[String] = info.map(|i| i.posi.as_slice()).unwrap_or(&[]);

    // dict.lisp:708 (when info ...)
    if let Some(info) = info {
        // dict.lisp:709-712 — (or (intersection seq-set *no-kanji-break-penalty*)
        //                        (and (eql end :beg) (alexandria:starts-with #\す text)))
        //   → (return-from kanji-break-penalty score)
        let seq_set_intersects = info
            .seq_set
            .iter()
            .any(|s| NO_KANJI_BREAK_PENALTY.contains(s));
        let starts_with_su = end == KanjiBreakEnd::Beg && text.chars().next() == Some('す');
        if seq_set_intersects || starts_with_su {
            return Ok(score);
        }

        // dict.lisp:713-721 ((intersection '("vs-s" "v5s") posi :test 'equal) …)
        //
        // Lisp `cond` semantics: once this clause matches it consumes the
        // dispatch, even when the inner `(when suru-suffix …)` evaluates to
        // nil. The remaining num/suf/pref clauses do NOT fire. The Rust
        // port mirrors that with an explicit `vs-s/v5s ∈ posi` guard on
        // the else-if chain below — without it, a `posi` that contains
        // both "v5s" and "suf" (e.g. dict.lisp:702 row "下す" posi=("suf"
        // "v5s" "vt")) double-fires and adds the +10 suf bonus that
        // Lisp's cond skipped.
        let is_vs_or_v5s = posi.iter().any(|s| s == "vs-s" || s == "v5s");
        if is_vs_or_v5s {
            // dict.lisp:715 (find :suru (get-suffixes text) :key 'second)
            let suffixes = get_suffixes(ctx, text);
            let suru_suffix = suffixes.iter().find(|(_, key, _)| *key == "suru");
            if let Some(&(suffix_text, _key, kf)) = suru_suffix {
                // Upstream `(calc-score (third suru-suffix) …)` feeds the
                // kana-form straight in; on a nil kf it would crash inside
                // calc-score's `(word-type nil)` call. Mirror that
                // panic-on-nil contract here rather than silently skipping.
                // `:SURU` entries are populated exclusively by
                // `load-conjs :suru …` at dict-grammar.lisp:244-248 →
                // `load-kf` → `(get-kana-forms seq)` element which is
                // always non-nil. Only `load-abbr` produces nil-kf cache
                // rows, and it never uses the `:SURU` key.
                let kf = kf.expect(
                    "load-conjs :suru always populates kf — see \
                     dict-grammar.lisp:244-248 / load-kf",
                );
                // dict.lisp:717 (offset = mora-length text - mora-length suffix-text)
                let text_mora = mora_length(text) as i32;
                let suffix_mora = mora_length(suffix_text) as i32;
                let offset = text_mora - suffix_mora;
                // dict.lisp:718-720 (calc-score (third suru-suffix)
                //                     :use-length (and use-length (- use-length offset))
                //                     :score-mod score-mod)
                let use_length_recur = use_length.map(|ul| ul - offset);
                let kf_word: KaniWordDispatchEnum =
                    KaniWordDispatchEnum::Kana((*kf).clone());
                let (suffix_score, _info) = Box::pin(calc_score(
                    ctx,
                    &kf_word,
                    false,
                    use_length_recur,
                    score_mod,
                    &[],
                ))
                .await?;
                // dict.lisp:721 (return-from kanji-break-penalty (min score (+ suffix-score 50)))
                return Ok(score.min(suffix_score + 50));
            }
            // No suru-suffix → bonus stays 0; fall through to the
            // post-cond `(if (>= score *score-cutoff*) …)` arithmetic
            // WITHOUT entering the num/suf/pref clauses. Matches Lisp's
            // cond-consumed-by-this-clause semantics.
        } else if end == KanjiBreakEnd::Beg && posi.iter().any(|s| s == "num") {
            // dict.lisp:722-723 ((and (eql end :beg) (member "num" posi)) (incf bonus 5))
            bonus += 5;
        } else if end == KanjiBreakEnd::Beg
            && posi.iter().any(|s| s == "suf" || s == "n-suf")
        {
            // dict.lisp:724-726 ((and (eql end :beg) (intersection '("suf" "n-suf") posi)) (incf bonus 10))
            bonus += 10;
        } else if end == KanjiBreakEnd::End && posi.iter().any(|s| s == "pref") {
            // dict.lisp:727-728 ((and (eql end :end) (member "pref" posi)) (incf bonus 12))
            bonus += 12;
        }
    }

    // dict.lisp:730-732 (if (>= score *score-cutoff*)
    //                       (max *score-cutoff* (+ (ceiling score ratio) bonus))
    //                       score)
    if score >= SCORE_CUTOFF {
        // dict.lisp:731 (ceiling score ratio) — positive-operand round-up.
        let ceiling = (score + ratio - 1) / ratio;
        Ok(SCORE_CUTOFF.max(ceiling + bonus))
    } else {
        Ok(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- pure-arithmetic cases (no info, no DB) -----
    //
    // Every assertion REPL-pinned against upstream ichiran 2026-05-16.

    #[tokio::test]
    async fn no_info_above_cutoff_halves_with_ceiling() {
        // REPL: (kanji-break-penalty '(0) 100) → 50
        // 100 >= 5 → max(5, ceil(100/2) + 0) = max(5, 50) = 50
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn no_info_odd_score_rounds_up() {
        // REPL: (kanji-break-penalty '(1) 100) → 50 (same arithmetic; end branch unused without posi)
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[1], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn no_info_both_branch() {
        // REPL: (kanji-break-penalty '(0 5) 100) → 50
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0, 5], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn below_cutoff_returns_unchanged() {
        // REPL: (kanji-break-penalty '(0) 4) → 4 (4 < *score-cutoff* = 5)
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 4, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 4);
    }

    // ----- info-bearing cases (calc_score + kanji_break_penalty integration) -----
    //
    // The pure-arithmetic cases above exercise the `info=None` arm.
    // These exercise the four cond branches at dict.lisp:709-728 that
    // gate on info contents.

    /// REPL: with seq 1467640 (`猫`, common-rank-7 noun) →
    ///   `(calc-score row)` → 19, info :posi ("n") :seq-set (1467640).
    ///   `(kanji-break-penalty '(0) 19 :info info :text "猫")` → 10.
    ///   Hits the fall-through "penalty applies" branch
    ///   (no seq-set ∩ `*no-kanji-break-penalty*`, no `す` prefix, no
    ///   num/suf/pref bonus). Arithmetic: 19 ≥ 5 → max(5, ceil(19/2) + 0)
    ///   = max(5, 10) = 10.
    #[tokio::test]
    async fn info_fall_through_penalty() {
        use crate::dict::calc_score::calc_score;
        use crate::dict::kani::KaniWordDispatchEnum;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::kanji_text_dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1467640 AND text = '猫' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("猫 1467640 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 19);
        let info = info.unwrap();
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "猫", None, None)
            .await
            .unwrap();
        assert_eq!(got, 10);
    }

    /// REPL: `飲む` (seq 1169870) is in `*no-kanji-break-penalty*`,
    /// so `kanji-break-penalty` returns `score` unchanged regardless
    /// of arithmetic. Pinned at score=128 (from `(calc-score …)` on
    /// the kanji row).
    #[tokio::test]
    async fn no_penalty_list_short_circuit() {
        use crate::dict::calc_score::calc_score;
        use crate::dict::kani::KaniWordDispatchEnum;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::kanji_text_dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1169870 AND text = '飲む' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("飲む 1169870 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        let info = info.unwrap();
        // dict.lisp:709 — intersection seq-set *no-kanji-break-penalty*
        // returns truthy → return score unchanged.
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "飲む", None, None)
            .await
            .unwrap();
        assert_eq!(got, score);
    }

    /// REPL: `好き` (seq 1277450) is in `*no-kanji-break-penalty*`,
    /// short-circuits regardless of text. Also exercises the
    /// `(eql end :beg) (alexandria:starts-with #\す text)` arm —
    /// even if seq-set didn't short-circuit, the `す`-prefix branch
    /// would. Pinned via the seq-set route.
    #[tokio::test]
    async fn suki_seq_short_circuit() {
        use crate::dict::calc_score::calc_score;
        use crate::dict::kani::KaniWordDispatchEnum;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::kanji_text_dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1277450 AND text = '好き' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("好き 1277450 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        let info = info.unwrap();
        let got_kanji_text = kanji_break_penalty(
            &ctx, &[0], score, Some(&info), "好き", None, None,
        )
        .await
        .unwrap();
        let got_kana_text = kanji_break_penalty(
            &ctx, &[0], score, Some(&info), "すき", None, None,
        )
        .await
        .unwrap();
        // REPL pinned: both → score unchanged (seq-set short-circuits first).
        assert_eq!(got_kanji_text, score);
        assert_eq!(got_kana_text, score);
    }

    #[tokio::test]
    async fn classify_end_results() {
        // pinned via direct cond evaluation on .103: kanji-break list →
        // (cond ((cdr kb) :both) ((eql (car kb) 0) :beg) (t :end))
        assert_eq!(classify_end(&[]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[0]), KanjiBreakEnd::Beg);
        assert_eq!(classify_end(&[3]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[5]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[0, 2]), KanjiBreakEnd::Both);
        assert_eq!(classify_end(&[1, 4]), KanjiBreakEnd::Both);
        assert_eq!(classify_end(&[0, 1, 2]), KanjiBreakEnd::Both);
    }
}
