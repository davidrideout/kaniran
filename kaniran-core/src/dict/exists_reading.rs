//! Port of `ichiran/dict:exists-reading` (`dict.lisp:1846`).
//!
//! ```lisp
//! (defun exists-reading (seq reading)
//!   (query (:select 'seq :from 'kana-text :where (:and (:= 'seq seq) (:= 'text reading)))))
//! ```
//!
//! Returns the `seq` of every `kana_text` row whose `seq` matches the
//! argument and whose `text` is `reading` — a non-empty result means
//! the reading is recorded for that entry. The sole upstream caller
//! (`find-word-info`, `dict.lisp:1866`) uses it as a predicate.
//!
//! Divergences from Lisp:
//! - Takes `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per [`crate::conn::kani_context`].
//! - The postmodern `:rows` result for the single-column select is a
//!   list of one-element rows (`((1376070))`); the port returns the
//!   flat [`Vec<i32>`] of `seq` values. Every row carries the input
//!   `seq`, so the flattening is lossless and non-emptiness is the
//!   existence signal.

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
