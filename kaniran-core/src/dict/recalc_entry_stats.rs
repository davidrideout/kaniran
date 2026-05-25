//! Port of `ichiran/dict:recalc-entry-stats` (`dict.lisp:55`).
//!
//! Recomputes the `n_kanji` / `n_kana` row-count caches on the `entry`
//! rows whose `seq` is in `entries`, from their current `kanji_text` /
//! `kana_text` children.
//!
//! Diverges from the upstream lambda list `(&rest entries)`:
//! - takes `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per [`crate::conn::kani_context`];
//! - `&rest entries` becomes the slice `entries: &[i32]`;
//! - `(:in 'entry.seq (:set entries))` binds as `entry.seq = ANY($1)`
//!   (postmodern → sqlx idiom; empty input affects 0 rows either way,
//!   matching upstream's `seq IN (NULL)`, REPL-pinned 2026-05-25);
//! - upstream `query` returns `(values nil affected-count)` for a
//!   no-RETURNING UPDATE; the port returns the affected-row count
//!   (`PgQueryResult::rows_affected`), postmodern's secondary value.
//!   Both callers discard it.

use crate::conn::kani_context::KaniranContext;

pub async fn recalc_entry_stats(
    ctx: &KaniranContext,
    entries: &[i32],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE entry SET \
         n_kanji = (SELECT COUNT(id) FROM kanji_text WHERE kanji_text.seq = entry.seq), \
         n_kana = (SELECT COUNT(id) FROM kana_text WHERE kana_text.seq = entry.seq) \
         WHERE entry.seq = ANY($1)",
    )
    .bind(entries)
    .execute(&ctx.pool)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Idempotent on a consistent dictionary: the UPDATE rewrites each
    // matched entry's n_kanji / n_kana to the same values it already
    // holds. Affected count is the number of matched entry rows, not
    // changed rows. All cases REPL-pinned against ichiran 2026-05-25.
    #[tokio::test]
    async fn affected_count_matches_matched_rows() {
        let ctx = KaniranContext::from_env().await.expect("ctx");

        // (recalc-entry-stats 1591050) -> 1 row affected.
        let one = recalc_entry_stats(&ctx, &[1591050]).await.expect("one");
        assert_eq!(one, 1);

        // (recalc-entry-stats 1591050 1495740 1221520) -> 3.
        let multi = recalc_entry_stats(&ctx, &[1591050, 1495740, 1221520])
            .await
            .expect("multi");
        assert_eq!(multi, 3);

        // (recalc-entry-stats) -> 0 (empty set, seq IN (NULL)).
        let empty = recalc_entry_stats(&ctx, &[]).await.expect("empty");
        assert_eq!(empty, 0);

        // A seq with no matching entry row -> 0 affected.
        let missing = recalc_entry_stats(&ctx, &[99999999]).await.expect("missing");
        assert_eq!(missing, 0);

        // (recalc-entry-stats 1591050 99999999) -> 1 (only the present
        // seq is matched; the absent one contributes nothing).
        let mixed = recalc_entry_stats(&ctx, &[1591050, 99999999]).await.expect("mixed");
        assert_eq!(mixed, 1);
    }

    #[tokio::test]
    async fn stats_match_child_counts_after_recalc() {
        let ctx = KaniranContext::from_env().await.expect("ctx");

        // Varied vocabulary spanning the kanji/kana count combinations.
        // seq -> (n_kanji, n_kana). REPL-pinned 2026-05-25.
        let cases: &[(i32, i32, i32)] = &[
            (1603990, 2, 1), // 仄か
            (1000580, 2, 2), // 彼
            (1582710, 1, 2),
            (2028930, 0, 1), // が
            (1467640, 1, 2),
        ];
        let seqs: Vec<i32> = cases.iter().map(|(seq, _, _)| *seq).collect();

        let affected = recalc_entry_stats(&ctx, &seqs).await.expect("recalc");
        assert_eq!(affected, seqs.len() as u64);

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
