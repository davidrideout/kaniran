//! Port of `ichiran/dict:entry-digest` (`dict.lisp:64`).
//!
//! ```lisp
//! (defun entry-digest (entry)
//!   (list (seq entry) (get-text entry) (get-kana entry)))
//! ```
//!
//! The 3-element list becomes a tuple; the text and kana elements are
//! `Option<String>` per the [`Entry::get_text`] / [`Entry::get_kana`] ports.

use super::entry_dao::Entry;
use crate::conn::kani_context::KaniranContext;

pub async fn entry_digest(
    ctx: &KaniranContext,
    entry: &Entry,
) -> Result<(i32, Option<String>, Option<String>), sqlx::Error> {
    Ok((
        entry.seq,
        entry.get_text(ctx).await?,
        entry.get_kana(ctx).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn load_entry(ctx: &KaniranContext, seq: i32) -> Entry {
        sqlx::query_as::<_, Entry>("SELECT * FROM entry WHERE seq = $1")
            .bind(seq)
            .fetch_one(&ctx.pool)
            .await
            .unwrap()
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-digest`), 2026-05-25.
    /// Covers `n-kanji > 0` entries (get-text reads kanji-text: a noun,
    /// a 2-kanji noun, a verb) and `n-kanji = 0` entries (get-text reads
    /// kana-text, so text equals kana: a katakana loanword and an
    /// onomatopoeic adverb).
    #[tokio::test]
    async fn entry_digest_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str, &str)] = &[
            (1257590, "憲法", "けんぽう"),
            (1386690, "雪崩", "なだれ"),
            (1573390, "躊躇う", "ためらう"),
            (1087690, "ドーナツ", "ドーナツ"),
            (1010900, "ぴったり", "ぴったり"),
        ];
        for (seq, text, kana) in cases {
            let entry = load_entry(&ctx, *seq).await;
            let digest = entry_digest(&ctx, &entry).await.unwrap();
            assert_eq!(
                (digest.0, digest.1.as_deref(), digest.2.as_deref()),
                (*seq, Some(*text), Some(*kana)),
                "seq={seq}"
            );
        }
    }
}
