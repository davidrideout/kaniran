//! Port of `ichiran/dict:next-seq` (`dict-load.lisp:112`).
//!
//! `MAX(seq) + 1` from `entry`. Panics if the table is empty

use crate::conn::kani_context::KaniranContext;

pub async fn next_seq(ctx: &KaniranContext) -> Result<i32, sqlx::Error> {
    let max: Option<i32> = sqlx::query_scalar("SELECT MAX(seq) FROM entry")
        .fetch_one(&ctx.pool)
        .await?;
    Ok(max.expect("next_seq: entry table is empty") + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::init_tables::init_tables;

    #[tokio::test]
    #[ignore = "DB test; requires KANIRAN_TEST_DATABASE_URL"]
    async fn next_seq_returns_max_plus_one() {
        let ctx = KaniranContext::pool_only_test_ctx().await;
        init_tables(&ctx).await.unwrap();
        // Two seeded rows mirror the upstream JMdict range; REPL
        // probe on .103 (2026-05-31) confirmed next-seq returns
        // MAX_SEQ + 1 (12297856 → 12297857) on the live corpus.
        sqlx::query(
            "INSERT INTO entry (seq, content, root_p, n_kanji, n_kana, primary_nokanji) \
             VALUES (1000000, '', TRUE, 0, 0, FALSE), \
                    (1000042, '', TRUE, 0, 0, FALSE)",
        )
        .execute(&ctx.pool)
        .await
        .unwrap();
        assert_eq!(next_seq(&ctx).await.unwrap(), 1000043);
    }
}
