//! Port of `ichiran/dict:word-readings` (`dict.lisp:536`).
//!
//! When `word` itself appears in `kana-text` the readings are just the
//! word; otherwise the kana spellings of every `kanji-text` entry with
//! that surface form, ordered by id. Returns the readings alongside
//! their romanizations.

use crate::conn::kani_context::KaniranContext;
use crate::core::_star_default_romanization_method_star_::default_romanization_method;
use crate::core::generic_romanization_class::RomanizationMethod;
use crate::core::romanize_word::romanize_word;

pub async fn word_readings(
    ctx: &KaniranContext,
    word: &str,
) -> Result<(Vec<String>, Vec<String>), sqlx::Error> {
    // dict.lisp:537 (kana-seq (query (:select 'seq :from 'kana-text :where (:= 'text word)) :column))
    let kana_seq: Vec<i32> = sqlx::query_scalar("SELECT seq FROM kana_text WHERE text = $1")
        .bind(word)
        .fetch_all(&ctx.pool)
        .await?;
    // dict.lisp:538-545 (readings (if kana-seq (list word) …))
    let readings: Vec<String> = if !kana_seq.is_empty() {
        vec![word.to_string()]
    } else {
        // dict.lisp:540-541 (kanji-seq (query (:select 'seq :from 'kanji-text :where (:= 'text word)) :column))
        let kanji_seq: Vec<i32> = sqlx::query_scalar("SELECT seq FROM kanji_text WHERE text = $1")
            .bind(word)
            .fetch_all(&ctx.pool)
            .await?;
        // dict.lisp:542-545 (query (:order-by (:select 'text :from 'kana-text :where (:in 'seq (:set kanji-seq))) 'id) :column)
        sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = ANY($1) ORDER BY id")
            .bind(&kanji_seq)
            .fetch_all(&ctx.pool)
            .await?
    };
    // dict.lisp:546 (values readings (mapcar #'ichiran:romanize-word readings))
    let method = RomanizationMethod::TraditionalHepburn(default_romanization_method());
    let romanizations: Vec<String> = readings
        .iter()
        .map(|reading| romanize_word(reading, method, None, true))
        .collect();
    Ok((readings, romanizations))
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL fixtures (.103, ichiran/dict:word-readings), 2026-05-25.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn word_readings_fixtures() {
        let ctx = ctx_from_env().await;
        // (word, readings, romanizations). Cases cover:
        // - kana branch (word is itself in kana-text): word returned verbatim;
        // - kanji branch (ORDER BY id over the kana spellings): single & multi;
        // - katakana kana-branch input; macron long vowels; empty-on-both.
        let cases: &[(&str, &[&str], &[&str])] = &[
            // kanji branch, multiple kana readings ordered by id.
            ("猫", &["ねこ", "ネコ", "ねこま"], &["neko", "neko", "nekoma"]),
            // kana branch — word is in kana-text, returned as-is.
            ("ねこ", &["ねこ"], &["neko"]),
            // kana branch, katakana surface form (long-vowel bar).
            ("コーヒー", &["コーヒー"], &["kohi"]),
            // kana branch, hiragana with macron long vowel.
            ("ありがとう", &["ありがとう"], &["arigatō"]),
            // kanji branch, single reading.
            ("図書館", &["としょかん"], &["toshokan"]),
            ("東京", &["とうきょう"], &["tōkyō"]),
            ("牛乳", &["ぎゅうにゅう"], &["gyūnyū"]),
            // mixed kanji+kana surface; not in kana-text, one kanji reading.
            ("食べる", &["たべる"], &["taberu"]),
            // not present in either table → empty kanji-seq → empty IN set.
            ("ヌルポポポ", &[], &[]),
        ];
        for (word, exp_readings, exp_rom) in cases {
            let (readings, romanizations) = word_readings(&ctx, word).await.unwrap();
            assert_eq!(&readings, exp_readings, "readings for {word}");
            assert_eq!(&romanizations, exp_rom, "romanizations for {word}");
        }
    }
}
