//! Port of `ichiran/dict:reading-str-seq` (`dict.lisp:1584`).
//!
//! Looks up the ord-0 kanji and kana surface forms for `seq` and formats
//! them via `reading-str*`.

use super::reading_str_star_::reading_str_star_;
use crate::conn::kani_context::KaniranContext;

pub async fn reading_str_seq(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Option<String>, sqlx::Error> {
    let kanji_text: Option<String> =
        sqlx::query_scalar("SELECT text FROM kanji_text WHERE seq = $1 AND ord = 0")
            .bind(seq)
            .fetch_optional(&ctx.pool)
            .await?;
    let kana_text: Option<String> =
        sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = $1 AND ord = 0")
            .bind(seq)
            .fetch_optional(&ctx.pool)
            .await?;
    Ok(reading_str_star_(kanji_text.as_deref(), kana_text.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::reading-str-seq`), 2026-05-24.
    /// Covers a kanji+kana entry, a conjugating kanji+kana verb, two
    /// kana-only entries (no kanji-text row → kanji nil → bare kana),
    /// and an unknown seq (both column queries empty → nil).
    #[tokio::test]
    async fn reading_str_seq_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>)] = &[
            (1582710, Some("日本 【にほん】")),
            (1358280, Some("食べる 【たべる】")),
            (1010890, Some("ぴちぴち")),
            (1056070, Some("サイバネーション")),
            (999999, None),
        ];
        for (seq, expected) in cases {
            assert_eq!(
                reading_str_seq(&ctx, *seq).await.unwrap().as_deref(),
                *expected,
                "seq={seq}"
            );
        }
    }
}
