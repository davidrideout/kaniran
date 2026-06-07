//! Port of `ichiran/kanji:process-match-json` (`kanji.lisp:428`).
//!
//! Walks the [`MatchedSegment`] list from [`super::match_readings`]:
//! each non-`irr` kanji segment becomes a [`super::kanji_reading_json`]
//! object, each non-kanji run a `{"text": …}` object, and consecutive
//! `irr` kanji segments coalesce into one `{"kanji", "reading", "type":
//! "irr"}` object whose texts are the concatenated segments, gaining
//! `"link": true` when the concatenation holds a CJK ideograph.

use std::sync::OnceLock;

use fancy_regex::Regex;
use serde_json::{Map, Value};

use super::get_reading_alternatives::ReadingTag;
use super::kanji_reading_json::kanji_reading_json;
use super::match_readings::MatchedSegment;
use crate::characters::_star_kanji_char_regex_star_::KANJI_CHAR_REGEX;
use crate::conn::kani_context::KaniranContext;

fn kanji_char_scanner() -> &'static Regex {
    static KANJI_CHAR_SCANNER: OnceLock<Regex> = OnceLock::new();
    KANJI_CHAR_SCANNER
        .get_or_init(|| Regex::new(KANJI_CHAR_REGEX).expect("*kanji-char-regex* must compile"))
}

// kanji.lisp:430-438 (empty-bag closure) — flush the accumulated irr segments
// into one "irr" object. Segments accumulate in original order here (Lisp
// pushes then `nreverse`s), so no reversal is needed.
fn empty_bag(irrbag: &mut Vec<(&str, &str)>, result: &mut Vec<Value>) {
    let kanji: String = irrbag.iter().map(|(kanji, _)| *kanji).collect();
    let reading: String = irrbag.iter().map(|(_, reading)| *reading).collect();
    let link = kanji_char_scanner()
        .is_match(&kanji)
        .expect("scan over fixed *kanji-char-regex* pattern cannot fail");
    let mut js = Map::new();
    js.insert("kanji".to_owned(), Value::String(kanji));
    js.insert("reading".to_owned(), Value::String(reading));
    js.insert("type".to_owned(), Value::String("irr".to_owned()));
    if link {
        js.insert("link".to_owned(), Value::Bool(true));
    }
    result.push(Value::Object(js));
    irrbag.clear();
}

pub async fn process_match_json(
    ctx: &KaniranContext,
    match_: &[MatchedSegment],
) -> Result<Vec<Value>, sqlx::Error> {
    let mut irrbag: Vec<(&str, &str)> = Vec::new();
    let mut result: Vec<Value> = Vec::new();
    for item in match_ {
        match item {
            // kanji.lisp:440-445 ((listp item) cond)
            MatchedSegment::Kanji { kanji, reading } => {
                if reading.r#type == "irr" {
                    irrbag.push((kanji.as_str(), reading.reading.as_str()));
                } else {
                    if !irrbag.is_empty() {
                        empty_bag(&mut irrbag, &mut result);
                    }
                    // kanji.lisp:445 ((apply 'kanji-reading-json item))
                    result.push(
                        kanji_reading_json(
                            ctx,
                            kanji,
                            &reading.reading,
                            &reading.r#type,
                            matches!(reading.tag, Some(ReadingTag::Rendaku)),
                            reading.gem.as_deref(),
                        )
                        .await?,
                    );
                }
            }
            // kanji.lisp:446-448 (else do (when irrbag (funcall empty-bag)) (push (jsown:new-js ("text" item)) result))
            MatchedSegment::NonKanji(text) => {
                if !irrbag.is_empty() {
                    empty_bag(&mut irrbag, &mut result);
                }
                let mut js = Map::new();
                js.insert("text".to_owned(), Value::String(text.clone()));
                result.push(Value::Object(js));
            }
        }
    }
    // kanji.lisp:449 (finally (when irrbag (funcall empty-bag)))
    if !irrbag.is_empty() {
        empty_bag(&mut irrbag, &mut result);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::match_readings::match_readings;
    use std::sync::Arc;

    async fn ctx() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `jsown:to-json` of `(process-match-json (match-readings str reading))`),
    /// 2026-05-25. Input segments are produced by [`match_readings`] on the source word.
    ///
    /// Covers: a kanji segment followed by a non-kanji `{"text"}` run
    /// (見る → 見/み, る); a geminated reading (学校 → 学/がっ gem く); a
    /// rendaku reading (三日月 → 月/づき); an all-irr word coalesced with a
    /// `link` (今日 → 今日/きょう); a single irr segment flushed mid-list
    /// between two normal kanji (明日香 → 明, [日 irr], 香); the iteration
    /// mark 々 which gets no `link` (人々 → 人, 々 rendaku, no link); and a
    /// geminate-then-rendaku pair (日本 → 日/にっ gem ち, 本/ぽん rendaku).
    #[tokio::test]
    async fn process_match_json_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, &str, &str)] = &[
            (
                "見る", "みる",
                r#"[{"kanji":"見","reading":"み","type":"ja_kun","link":true,"stats":true,"sample":135,"total":173,"perc":"78.03%","grade":1},{"text":"る"}]"#,
            ),
            (
                "学校", "がっこう",
                r#"[{"kanji":"学","reading":"がっ","type":"ja_on","link":true,"geminated":"く","stats":true,"sample":214,"total":216,"perc":"99.07%","grade":1},{"kanji":"校","reading":"こう","type":"ja_on","link":true,"stats":true,"sample":51,"total":51,"perc":"100.00%","grade":1}]"#,
            ),
            (
                "三日月", "みかづき",
                r#"[{"kanji":"三","reading":"み","type":"ja_kun","link":true,"stats":true,"sample":3,"total":89,"perc":"3.37%","grade":1},{"kanji":"日","reading":"か","type":"ja_kun","link":true,"stats":true,"sample":19,"total":263,"perc":"7.22%","grade":1},{"kanji":"月","reading":"づき","type":"ja_kun","link":true,"rendaku":"RENDAKU","stats":true,"sample":18,"total":93,"perc":"19.35%","grade":1}]"#,
            ),
            (
                "今日", "きょう",
                r#"[{"kanji":"今日","reading":"きょう","type":"irr","link":true}]"#,
            ),
            (
                "明日香", "あすか",
                r#"[{"kanji":"明","reading":"あ","type":"ja_kun","link":true,"stats":true,"sample":17,"total":85,"perc":"20.00%","grade":2},{"kanji":"日","reading":"す","type":"irr","link":true},{"kanji":"香","reading":"か","type":"ja_kun","link":true,"stats":true,"sample":0,"total":15,"perc":"0.00%","grade":8}]"#,
            ),
            (
                "人々", "ひとびと",
                r#"[{"kanji":"人","reading":"ひと","type":"ja_kun","link":true,"stats":true,"sample":47,"total":345,"perc":"13.62%","grade":1},{"kanji":"々","reading":"びと","type":"ja_kun","rendaku":"RENDAKU"}]"#,
            ),
            (
                "日本", "にっぽん",
                r#"[{"kanji":"日","reading":"にっ","type":"ja_on","link":true,"geminated":"ち","stats":true,"sample":90,"total":263,"perc":"34.22%","grade":1},{"kanji":"本","reading":"ぽん","type":"ja_on","link":true,"rendaku":"RENDAKU","stats":true,"sample":173,"total":177,"perc":"97.74%","grade":1}]"#,
            ),
        ];
        for (str, reading, expected) in cases {
            let match_ = match_readings(&ctx, str, reading)
                .await
                .unwrap()
                .expect("match-readings aligns");
            let result = process_match_json(&ctx, &match_).await.unwrap();
            let actual = serde_json::to_string(&result).unwrap();
            assert_eq!(actual.as_str(), *expected, "str={str} reading={reading}");
        }
    }
}
