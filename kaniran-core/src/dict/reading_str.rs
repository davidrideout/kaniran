//! Port of `ichiran/dict:reading-str` (gf — `dict.lisp:1588-1592`, `1739-1743`).
//!
//! ```lisp
//! (defgeneric reading-str (obj)
//!   (:method ((obj simple-text)) (reading-str* (get-kanji obj) (get-kana obj)))
//!   (:method ((obj integer)) (reading-str-seq obj)))
//! (defmethod reading-str ((word-info word-info)) (word-info-reading-str word-info))
//! (defmethod reading-str ((word-info list)) (word-info-reading-str word-info))
//! ```
//!
//! The `integer` method is the existing
//! [`super::reading_str_seq::reading_str_seq`]; callers with an integer seq
//! invoke it directly. The `list` method shares the `word-info` body (a
//! jsown alist and a CLOS `word-info` are the same reading upstream, both
//! the [`WordInfo`] struct in the port). The `simple-text` method takes
//! `&KaniranContext` for the DB-backed `get-kanji` / `get-kana`.

use super::get_kanji::get_kanji;
use super::reading_str_star_::reading_str_star_;
use super::word_info_class::WordInfo;
use super::word_info_reading_str::word_info_reading_str;
use crate::conn::kani_context::KaniranContext;
use crate::dict::kani::KaniSimpleTextDispatchEnum;

impl WordInfo {
    // dict.lisp:1739-1743 (defmethod reading-str ((word-info word-info)) / ((word-info list)))
    pub fn reading_str(&self) -> Option<String> {
        word_info_reading_str(self)
    }
}

impl KaniSimpleTextDispatchEnum {
    // dict.lisp:1589-1590 (defmethod reading-str ((obj simple-text)))
    pub async fn reading_str(
        &self,
        ctx: &KaniranContext,
    ) -> Result<Option<String>, sqlx::Error> {
        let kanji = get_kanji(ctx, &self.to_word()).await?;
        let kana = self.get_kana(ctx).await?;
        Ok(reading_str_star_(kanji.as_deref(), kana.as_deref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_simple_text(ctx: &KaniranContext, word: &str) -> KaniSimpleTextDispatchEnum {
        match find_word(ctx, word, false).await.unwrap() {
            FindWordRows::Kanji(rows) => KaniSimpleTextDispatchEnum::Kanji(
                rows.into_iter().next().expect("at least one kanji-text row"),
            ),
            FindWordRows::Kana(rows) => KaniSimpleTextDispatchEnum::Kana(
                rows.into_iter().next().expect("at least one kana-text row"),
            ),
        }
    }

    /// REPL fixtures (.103, `(reading-str (car (find-word W)))`), 2026-05-24.
    /// Covers the simple-text method: kanji-text (kanji+kana), kana-text with
    /// a kanji form (get-kanji via best-kanji-conj → 猫), and kana-text without
    /// one (get-kanji nil → bare kana).
    #[tokio::test]
    async fn reading_str_simple_text_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            ("日本", "日本 【にほん】"),
            ("ねこ", "猫 【ねこ】"),
            ("ぴちぴち", "ぴちぴち"),
            ("食べる", "食べる 【たべる】"),
            ("東京", "東京 【とうきょう】"),
        ];
        for (word, expected) in cases {
            let obj = first_simple_text(&ctx, word).await;
            assert_eq!(
                obj.reading_str(&ctx).await.unwrap().as_deref(),
                Some(*expected),
                "word={word}"
            );
        }
    }

    /// REPL fixtures (.103, `(reading-str (car (simple-segment S)))`), 2026-05-24.
    /// Covers the word-info method on real segmented (kanji-type) word-infos.
    #[tokio::test]
    async fn reading_str_word_info_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            ("東京", "東京 【とうきょう】"),
            ("日本語", "日本語 【にほんご】"),
        ];
        for (sentence, expected) in cases {
            let word_infos = crate::dict::simple_segment::simple_segment(&ctx, sentence, None)
                .await
                .unwrap();
            assert_eq!(
                word_infos[0].reading_str().as_deref(),
                Some(*expected),
                "sentence={sentence}"
            );
        }
    }
}
