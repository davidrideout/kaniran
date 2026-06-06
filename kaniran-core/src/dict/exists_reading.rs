//! Port of `ichiran/dict:exists-reading` (`dict.lisp:1846`).
//!
//! Returns the `seq` of every `kana_text` row matching `(seq, reading)`
//! — a non-empty result means the reading is recorded for that entry.

use crate::conn::kani_context::KaniranContext;
use sqlx::Row;

pub async fn exists_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
) -> Result<Vec<i32>, sqlx::Error> {
    let rows = sqlx::query("SELECT seq FROM kana_text WHERE seq = $1 AND text = $2")
        .bind(seq)
        .bind(reading)
        .fetch_all(&ctx.pool)
        .await?;
    Ok(rows.into_iter().map(|row| row.get::<i32, _>("seq")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL (.103, `ichiran/dict::exists-reading`) + local DB, 2026-05-24:
    /// 政府 seq 1376070 has kana-text row "せいふ"
    /// (`(exists-reading 1376070 "せいふ")` -> `((1376070))`); reading
    /// "ありえない" is absent for that seq (-> `NIL`); 猫 seq 1467640
    /// has kana-text row "ねこ".
    #[tokio::test]
    async fn reading_present_and_absent() {
        let ctx = ctx().await;
        assert_eq!(
            exists_reading(&ctx, 1376070, "せいふ").await.unwrap(),
            vec![1376070]
        );
        assert!(exists_reading(&ctx, 1376070, "ありえない")
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            exists_reading(&ctx, 1467640, "ねこ").await.unwrap(),
            vec![1467640]
        );
        // reading belongs to a different entry -> no row for this seq
        assert!(exists_reading(&ctx, 1467640, "せいふ")
            .await
            .unwrap()
            .is_empty());
    }
}
