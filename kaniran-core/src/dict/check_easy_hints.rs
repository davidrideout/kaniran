//! Port of `ichiran/dict:check-easy-hints` (`dict-split.lisp:906-914`).
//!
//! Test helper that scans every registered easy-hint kana row whose
//! kanji and kana readings fail to align under `match-readings`, and
//! returns those readings as `(reading, kanji, kana)` triples.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_easy_hints_seqs_star_::easy_hints_seqs;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::true_kana::true_kana;
use crate::dict::true_kanji::true_kanji;
use crate::kanji::matching::match_readings;

#[derive(Debug, Clone)]
pub struct CheckEasyHintsFailure {
    pub reading: KanaText,
    pub kanji: Option<String>,
    /// `None` mirrors upstream's `(text nil)` no-kana-row case
    /// (`get-kana` raises CL condition; Rust port surfaces None).
    /// A None `kana` is itself an alignment failure — recorded
    /// alongside true-kanji / true-kana misalignments.
    pub kana: Option<String>,
}

pub async fn check_easy_hints(
    ctx: &KaniranContext,
) -> Result<Vec<CheckEasyHintsFailure>, sqlx::Error> {
    // dict-split.lisp:908 — (select-dao 'kana-text (:in 'seq (:set *easy-hints-seqs*)))
    // Upstream uses a single `:in (:set ...)` clause. Postgres parameterized arrays
    // are equivalent — bind a single `&[i32]` and let sqlx generate the
    // `seq = ANY($1)` form. (sqlx::postgres doesn't expose IN-list directly.)
    let readings: Vec<KanaText> = sqlx::query_as(
        "SELECT * FROM kana_text WHERE seq = ANY($1)",
    )
    .bind(easy_hints_seqs())
    .fetch_all(&ctx.pool)
    .await?;

    // dict-split.lisp:909 — (let ((*disable-hints* t))) wraps the
    // entire loop body, covering true-kanji, true-kana, and
    // match-readings. Rebind the ctx once before the loop so all
    // three operations see the same binding (matches upstream
    // scope; today true-kanji + match-readings don't reach the
    // recursion guard so the binding is inert there, but the
    // scope must still match for any future code path that does).
    let ctx2 = ctx.with_disable_hints(true);
    let mut failures = Vec::new();
    for reading in readings {
        let lifted = KaniWordDispatchEnum::Kana(reading.clone());
        // dict-split.lisp:911 — (kanji = (true-kanji reading))
        let kanji = true_kanji(&ctx2, &lifted).await?;
        // dict-split.lisp:912 — (kana = (true-kana reading))
        let kana = true_kana(&ctx2, &lifted).await?;
        // dict-split.lisp:913-914 — (match = (match-readings kanji kana))
        // (unless match collect (list reading kanji kana))
        // Both `kanji` and `kana` can be None. Upstream behavior:
        // - `(match-readings nil kana)` returns nil (verified via
        //   kanji.lisp: `(make-rmap nil)` is nil, then
        //   `(match-readings* nil kana)` returns `:none`, outer
        //   `(unless (eql match :none) ...)` returns nil).
        // - `(match-readings kanji nil)` would raise on the inner
        //   `(length reading)` reading nil. The Rust None case for
        //   `kana` mirrors that by skipping the call and recording
        //   the misalignment.
        let match_result = match (&kanji, &kana) {
            (Some(k), Some(ka)) => match_readings(&ctx2, k, ka).await?,
            _ => None,
        };
        if match_result.is_none() {
            failures.push(CheckEasyHintsFailure {
                reading,
                kanji,
                kana,
            });
        }
    }
    Ok(failures)
}
