//! Port of `ichiran/dict:recalc-entry-stats-all` (`dict.lisp:61`).
//!
//! Recomputes the `n_kanji` / `n_kana` row-count caches on every
//! `entry` from its current `kanji_text` / `kana_text` children.

use crate::conn::kani_context::KaniranContext;

pub async fn recalc_entry_stats_all(ctx: &KaniranContext) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE entry SET \
         n_kanji = (SELECT COUNT(id) FROM kanji_text WHERE kanji_text.seq = entry.seq), \
         n_kana = (SELECT COUNT(id) FROM kana_text WHERE kana_text.seq = entry.seq)",
    )
    .execute(&ctx.pool)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Idempotent on a consistent dictionary: every entry's stored
    // n_kanji / n_kana already equals its child-row counts, so the
    // UPDATE rewrites them to the same values. Affected count is the
    // total entry row count regardless (Postgres counts matched rows,
    // not changed rows). REPL-pinned against ichiran 2026-05-25.
    #[tokio::test]
    async fn affects_all_entries_and_stats_match_children() {
        let ctx = KaniranContext::from_env().await.expect("ctx");

        let affected = recalc_entry_stats_all(&ctx).await.expect("recalc-all");

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entry")
            .fetch_one(&ctx.pool)
            .await
            .expect("count entries");
        assert_eq!(affected, total as u64, "affected != total entry rows");

        // Spot-check varied vocabulary post-recalc: stored stats equal
        // the independently-counted child rows. (REPL 2026-05-25.)
        // seq -> (n_kanji, n_kana)
        let cases: &[(i32, i32, i32)] = &[
            (1603990, 2, 1), // 仄か
            (1000580, 2, 2), // 彼
            (1582710, 1, 2),
            (1591050, 2, 1), // 気が付く
            (2028930, 0, 1), // が
            (1467640, 1, 2),
        ];
        for (seq, exp_kanji, exp_kana) in cases {
            let (n_kanji, n_kana): (i32, i32) =
                sqlx::query_as("SELECT n_kanji, n_kana FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("entry row");
            assert_eq!(n_kanji, *exp_kanji, "seq={seq} n_kanji");
            assert_eq!(n_kana, *exp_kana, "seq={seq} n_kana");

            let actual_kanji: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kanji_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("kanji count");
            let actual_kana: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kana_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("kana count");
            assert_eq!(n_kanji as i64, actual_kanji, "seq={seq} stored vs actual kanji");
            assert_eq!(n_kana as i64, actual_kana, "seq={seq} stored vs actual kana");
        }
    }
}
