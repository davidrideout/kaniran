//! Port of `ichiran/kanji:kanji-reading-json` (`kanji.lisp:410`).
//!
//! Builds a JSON object describing one kanji/reading/type triple, adding
//! `link`, `rendaku`, `geminated`, and corpus `stats` fields when they
//! apply.

use std::sync::OnceLock;

use fancy_regex::Regex;
use serde_json::{Map, Value};

use super::get_original_reading::get_original_reading;
use super::get_reading_stats::get_reading_stats;
use crate::characters::constants::KANJI_CHAR_REGEX;
use crate::conn::kani_context::KaniranContext;

fn kanji_char_scanner() -> &'static Regex {
    static KANJI_CHAR_SCANNER: OnceLock<Regex> = OnceLock::new();
    KANJI_CHAR_SCANNER
        .get_or_init(|| Regex::new(KANJI_CHAR_REGEX).expect("*kanji-char-regex* must compile"))
}

pub async fn kanji_reading_json(
    ctx: &KaniranContext,
    kanji: &str,
    reading: &str,
    r#type: &str,
    rendaku: bool,
    geminated: Option<&str>,
) -> Result<Value, sqlx::Error> {
    let mut js = Map::new();
    js.insert("kanji".to_owned(), Value::String(kanji.to_owned()));
    js.insert("reading".to_owned(), Value::String(reading.to_owned()));
    js.insert("type".to_owned(), Value::String(r#type.to_owned()));
    // kanji.lisp:412 ((ppcre:scan *kanji-char-regex* kanji))
    if kanji_char_scanner()
        .is_match(kanji)
        .expect("scan over fixed *kanji-char-regex* pattern cannot fail")
    {
        js.insert("link".to_owned(), Value::Bool(true));
    }
    if rendaku {
        // kanji.lisp:415 ((jsown:extend-js js ("rendaku" rendaku))) — jsown renders :rendaku as "RENDAKU"
        js.insert("rendaku".to_owned(), Value::String("RENDAKU".to_owned()));
    }
    if let Some(geminated) = geminated {
        js.insert("geminated".to_owned(), Value::String(geminated.to_owned()));
    }
    // kanji.lisp:418 ((get-reading-stats kanji (get-original-reading reading rendaku geminated) type))
    let stats = get_reading_stats(
        ctx,
        kanji,
        &get_original_reading(reading, rendaku, geminated),
        r#type,
    )
    .await?;
    if let Some((sample, total, perc, grade)) = stats {
        js.insert("stats".to_owned(), Value::Bool(true));
        js.insert("sample".to_owned(), Value::Number(sample.into()));
        js.insert("total".to_owned(), Value::Number(total.into()));
        js.insert("perc".to_owned(), Value::String(perc));
        // kanji.lisp:423 ((when (not (eql grade :null)) ...))
        if let Some(grade) = grade {
            js.insert("grade".to_owned(), Value::Number(grade.into()));
        }
    }
    Ok(Value::Object(js))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `jsown:to-json` of `(apply 'kanji-reading-json item)`
    /// for items produced by `match-readings` on the source word), 2026-05-24.
    /// Source words: 人々/ひとびと, 学校/がっこう, 三日月/みかづき, 日本/にっぽん.
    /// The 唖/あ row is a real null-grade `kanji`/`reading` row pinned via the DB
    /// directly (`load-kanji-stats` only updates grade≤8 kanji, so null-grade rows
    /// keep `stat_common`=0 → `--.--%` and emit no `grade` field).
    ///
    /// Covers: link present (人,学,月,本,唖) vs absent (々 iteration mark, U+3005);
    /// rendaku tag (々,月,本) vs none; geminated graft (学) vs none; stats present
    /// with grade (人,学,月,本), stats absent (々), stats present grade-`:null` (唖);
    /// and `get-original-reading` paths — identity, rendaku-strip (月), handakuten
    /// rendaku-strip (本: ぽん→ほん), geminated graft (学: がっ→がく).
    #[tokio::test]
    async fn kanji_reading_json_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, &str, &str, bool, Option<&str>, &str)] = &[
            (
                "人", "ひと", "ja_kun", false, None,
                r#"{"kanji":"人","reading":"ひと","type":"ja_kun","link":true,"stats":true,"sample":47,"total":345,"perc":"13.62%","grade":1}"#,
            ),
            (
                "々", "びと", "ja_kun", true, None,
                r#"{"kanji":"々","reading":"びと","type":"ja_kun","rendaku":"RENDAKU"}"#,
            ),
            (
                "学", "がっ", "ja_on", false, Some("く"),
                r#"{"kanji":"学","reading":"がっ","type":"ja_on","link":true,"geminated":"く","stats":true,"sample":214,"total":216,"perc":"99.07%","grade":1}"#,
            ),
            (
                "月", "づき", "ja_kun", true, None,
                r#"{"kanji":"月","reading":"づき","type":"ja_kun","link":true,"rendaku":"RENDAKU","stats":true,"sample":18,"total":93,"perc":"19.35%","grade":1}"#,
            ),
            (
                "本", "ぽん", "ja_on", true, None,
                r#"{"kanji":"本","reading":"ぽん","type":"ja_on","link":true,"rendaku":"RENDAKU","stats":true,"sample":173,"total":177,"perc":"97.74%","grade":1}"#,
            ),
            (
                "唖", "あ", "ja_on", false, None,
                r#"{"kanji":"唖","reading":"あ","type":"ja_on","link":true,"stats":true,"sample":0,"total":0,"perc":"--.--%"}"#,
            ),
        ];
        for (kanji, reading, r#type, rendaku, geminated, expected) in cases {
            let js = kanji_reading_json(&ctx, kanji, reading, r#type, *rendaku, *geminated)
                .await
                .unwrap();
            let actual = serde_json::to_string(&js).unwrap();
            assert_eq!(actual.as_str(), *expected, "kanji={kanji} reading={reading}");
        }
    }
}
