//! Port of `ichiran/dict:get-candidates` (`dict.lisp:1902`).
//!
//! Returns the `entry.seq` rows matching `(text, reading)`. When
//! `text` is pure kana per [`crate::characters::test_word`], the
//! query restricts to `root_p` kana-only entries (`k.text IS NULL`)
//! whose primary kana row equals `text`. Otherwise the query treats
//! `text` as a kanji writing and requires both the primary kanji row
//! and the primary kana row to match.
//!
//! Diverges from the upstream lambda list `(text reading)` only by:
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - taking `reading: Option<&str>` to express the upstream `nil`
//!   reading directly — the kanji branch then binds SQL NULL (no
//!   rows, mirroring the upstream `(:= 'r.text nil)` form).

use crate::characters::char_classes::CharClass;
use crate::characters::char_classes::test_word;
use crate::conn::kani_context::KaniranContext;

pub async fn get_candidates(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
) -> Result<Vec<i32>, sqlx::Error> {
    let is_kana = test_word(text, CharClass::Kana);
    if is_kana {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT e.seq FROM entry e \
             LEFT JOIN kana_text r ON e.seq = r.seq \
             LEFT JOIN kanji_text k ON e.seq = k.seq \
             WHERE e.root_p AND k.text IS NULL AND r.text = $1 AND r.ord = 0 \
             ORDER BY e.seq",
        )
        .bind(text)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    } else {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT e.seq FROM entry e \
             LEFT JOIN kana_text r ON e.seq = r.seq \
             LEFT JOIN kanji_text k ON e.seq = k.seq \
             WHERE k.text = $1 AND k.ord = 0 AND r.text = $2 AND r.ord = 0 \
             ORDER BY e.seq",
        )
        .bind(text)
        .bind(reading)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL probe pinned 2026-05-22 covers both branches and the no-row
    //! cases:
    //!   `(get-candidates "する" nil)` → `NIL` (kana branch, no root-p kana-only row).
    //!   `(get-candidates "漢字" "かんじ")` → `(1213170)` (kanji branch).
    //!   `(get-candidates "テスト" nil)` → `(1079760)` (kana branch hit).
    //!   `(get-candidates "ジャバスクリプトーー" nil)` → `NIL` (kana, no match).
    //!   `(get-candidates "漢字" "ZZZZZZZZ")` → `NIL` (kanji, bogus reading).
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn kana_branch_no_root_kana_only_entry() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "する", None).await.unwrap();
        assert!(out.is_empty(), "expected NIL, got {:?}", out);
    }

    #[tokio::test]
    async fn kanji_branch_with_reading() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "漢字", Some("かんじ")).await.unwrap();
        assert_eq!(out, vec![1213170]);
    }

    #[tokio::test]
    async fn kana_branch_pure_katakana_hit() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "テスト", None).await.unwrap();
        assert_eq!(out, vec![1079760]);
    }

    #[tokio::test]
    async fn kana_branch_unknown_kana_returns_empty() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "ジャバスクリプトーー", None)
            .await
            .unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn kanji_branch_bogus_reading_returns_empty() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "漢字", Some("ZZZZZZZZ")).await.unwrap();
        assert!(out.is_empty());
    }
}
