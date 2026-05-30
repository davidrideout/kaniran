//! Port of `ichiran/dict:word-info-reading` (`dict.lisp:1445`).
//!
//! ```lisp
//! (defun word-info-reading (word-info)
//!   (let ((table (case (word-info-type word-info) (:kanji 'kanji-text) (:kana 'kana-text)))
//!         (true-text (word-info-true-text word-info)))
//!     (when (and table true-text)
//!       (car (select-dao table (:= 'text true-text))))))
//! ```
//!
//! Looks up the reading DAO backing a [`WordInfo`]: the first
//! `kanji_text` row for a `:kanji` word-info, the first `kana_text`
//! row for a `:kana` one, matched on `text = true-text`. Returns
//! `None` when the type is `:gap`, `true-text` is nil, or no row
//! matches.
//!
//! Diverges from the upstream lambda list `(word-info)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`]. The
//! kanji-text / kana-text result is wrapped into [`KaniWordDispatchEnum`]
//! so the polymorphic reading consumers (`get-senses-json`'s
//! reading-getter) dispatch over it as upstream does over the bare DAO.

use super::kana_text_dao::KanaText;
use super::kani::KaniWordDispatchEnum;
use super::kanji_text_dao::KanjiText;
use super::word_info_class::{WordInfo, WordInfoType};
use crate::conn::kani_context::KaniranContext;

pub async fn word_info_reading(
    ctx: &KaniranContext,
    word_info: &WordInfo,
) -> Result<Option<KaniWordDispatchEnum>, sqlx::Error> {
    // (true-text (word-info-true-text word-info)) — the `(and table true-text)`
    // guard fails outright when true-text is nil.
    let true_text = match &word_info.true_text {
        Some(true_text) => true_text,
        None => return Ok(None),
    };
    // (case (word-info-type word-info) (:kanji 'kanji-text) (:kana 'kana-text))
    // then (car (select-dao table (:= 'text true-text)))
    match word_info.kind {
        WordInfoType::Kanji => {
            let row: Option<KanjiText> = sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1")
                .bind(true_text)
                .fetch_optional(&ctx.pool)
                .await?;
            Ok(row.map(KaniWordDispatchEnum::Kanji))
        }
        WordInfoType::Kana => {
            let row: Option<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE text = $1")
                .bind(true_text)
                .fetch_optional(&ctx.pool)
                .await?;
            Ok(row.map(KaniWordDispatchEnum::Kana))
        }
        // (case …) has no :gap clause → table nil → guard fails → nil.
        WordInfoType::Gap => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn wi(kind: WordInfoType, true_text: Option<&str>) -> WordInfo {
        WordInfo {
            kind,
            true_text: true_text.map(str::to_owned),
            ..Default::default()
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::word-info-reading`), 2026-05-25.
    /// Each true-text below has exactly one row in its table, so the
    /// `car` of `select-dao` is deterministic. Covers: the `:kanji`
    /// branch (学校, 図書館), the `:kana` branch (ねこ, きそうてんがい),
    /// the `:gap` type (table nil → None), nil true-text (guard fails →
    /// None), and a true-text with no matching row (select empty → None).
    #[tokio::test]
    async fn word_info_reading_fixtures() {
        let ctx = ctx_from_env().await;

        let cases: &[(WordInfo, Option<(i32, i32, bool)>)] = &[
            // (word-info, Some((seq, id, is_kanji)) | None)
            (wi(WordInfoType::Kanji, Some("学校")), Some((1206730, 9064, true))),
            (wi(WordInfoType::Kanji, Some("図書館")), Some((1370420, 29808, true))),
            (wi(WordInfoType::Kana, Some("ねこ")), Some((1467640, 54168, false))),
            (
                wi(WordInfoType::Kana, Some("きそうてんがい")),
                Some((1219430, 28651, false)),
            ),
            (wi(WordInfoType::Gap, Some("学校")), None),
            (wi(WordInfoType::Kanji, None), None),
            (wi(WordInfoType::Kanji, Some("存在しない漢字列 abcxyz")), None),
        ];

        for (word_info, expected) in cases {
            let result = word_info_reading(&ctx, word_info).await.unwrap();
            match (expected, result) {
                (None, None) => {}
                (Some((seq, id, is_kanji)), Some(KaniWordDispatchEnum::Kanji(row))) => {
                    assert!(*is_kanji, "true_text={:?}: expected kana-text", word_info.true_text);
                    assert_eq!(row.seq, *seq, "true_text={:?}", word_info.true_text);
                    assert_eq!(row.id, *id, "true_text={:?}", word_info.true_text);
                    assert_eq!(&row.text, word_info.true_text.as_ref().unwrap());
                }
                (Some((seq, id, is_kanji)), Some(KaniWordDispatchEnum::Kana(row))) => {
                    assert!(!*is_kanji, "true_text={:?}: expected kanji-text", word_info.true_text);
                    assert_eq!(row.seq, *seq, "true_text={:?}", word_info.true_text);
                    assert_eq!(row.id, *id, "true_text={:?}", word_info.true_text);
                    assert_eq!(&row.text, word_info.true_text.as_ref().unwrap());
                }
                (expected, result) => panic!(
                    "true_text={:?}: expected {expected:?}, got variant mismatch ({})",
                    word_info.true_text,
                    result.is_some()
                ),
            }
        }
    }
}
