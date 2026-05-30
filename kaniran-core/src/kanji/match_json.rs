//! Per-match-segment JSON rendering and the original-reading helper
//! that prepares input for it. From `kanji.lisp:308-330, 399-456`.

use std::sync::OnceLock;

use fancy_regex::Regex;
use serde_json::{Map, Value};

use crate::characters::char_classes::{KANJI_CHAR_REGEX, KANJI_REGEX};
use crate::characters::voicing::unrendaku;
use crate::conn::kani_context::KaniranContext;

use super::kanji_json::calculate_perc;
use super::matching::{match_readings, MatchedSegment};
use super::readings::ReadingTag;

/// `get-original-reading` (`kanji.lisp:308`). Recover the underlying
/// kun/on dictionary form from a reading variant: strip dakuten via
/// [`unrendaku`] when `rendaku`, and replace the trailing character
/// with the first char of `geminated` when present.
pub fn get_original_reading(
    rtext: &str,
    rendaku: bool,
    geminated: Option<&str>,
) -> String {
    let mut s = rtext.to_string();
    if rendaku {
        unrendaku(&mut s);
    }
    if let Some(g) = geminated {
        // kanji.lisp:311 ((setf (char rtext (1- (length rtext))) (char geminated 0)))
        let new_first = g.chars().next().expect("geminated is non-empty when present");
        let last_pos = s
            .char_indices()
            .last()
            .expect("rtext is non-empty when geminated is set")
            .0;
        let mut buf = [0u8; 4];
        let new_str = new_first.encode_utf8(&mut buf);
        s.replace_range(last_pos.., new_str);
    }
    s
}

/// `get-reading-stats` (`kanji.lisp:399`). Joins `kanji` and `reading`,
/// filters by `kanji.text`, `reading.text`, `reading.type`, and returns
/// `(reading.stat_common, kanji.stat_common, perc, kanji.grade)` or
/// `None`. `perc` is rendered via [`calculate_perc`]; `grade` stays
/// `Option<i32>` because the column is nullable.
pub async fn get_reading_stats(
    ctx: &KaniranContext,
    kanji: &str,
    reading: &str,
    r#type: &str,
) -> Result<Option<(i32, i32, String, Option<i32>)>, sqlx::Error> {
    // kanji.lisp:401 ((:select 'r.stat-common 'k.stat-common 'k.grade ... :row))
    let row = sqlx::query_as::<_, (i32, i32, Option<i32>)>(
        "SELECT r.stat_common, k.stat_common, k.grade FROM kanji k, reading r \
         WHERE k.id = r.kanji_id AND k.text = $1 AND r.text = $2 AND r.type = $3",
    )
    .bind(kanji)
    .bind(reading)
    .bind(r#type)
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .next();
    Ok(row.map(|(sample, total, grade)| {
        (sample, total, calculate_perc(sample, total), grade)
    }))
}

fn kanji_char_scanner() -> &'static Regex {
    static CACHE: OnceLock<Regex> = OnceLock::new();
    CACHE.get_or_init(|| Regex::new(KANJI_CHAR_REGEX).expect("*kanji-char-regex* must compile"))
}

fn kanji_scanner() -> &'static Regex {
    static CACHE: OnceLock<Regex> = OnceLock::new();
    CACHE.get_or_init(|| Regex::new(KANJI_REGEX).expect("*kanji-regex* must compile"))
}

/// `kanji-reading-json` (`kanji.lisp:410`). Per-(kanji, reading, type)
/// JSON object. Adds `link: true` when `kanji` holds a CJK ideograph,
/// `rendaku: "RENDAKU"` when set (jsown renders the `:rendaku` keyword
/// value that way), and `geminated` when present. When
/// [`get_reading_stats`] returns a row, adds `stats: true`, `sample`,
/// `total`, `perc`, and `grade` (when non-NULL).
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
        // kanji.lisp:415 — jsown renders :rendaku as "RENDAKU"
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

/// Flush the accumulated irr segments into one `"irr"` object.
/// Segments accumulate in original order here (Lisp pushes then
/// `nreverse`s), so no reversal is needed.
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

/// `process-match-json` (`kanji.lisp:428`). Walk the [`MatchedSegment`]
/// list: each non-`irr` kanji segment becomes a [`kanji_reading_json`]
/// object, each non-kanji run becomes `{"text": …}`, and consecutive
/// `irr` kanji segments coalesce into one
/// `{"kanji", "reading", "type": "irr"}` object whose texts are the
/// concatenated segments (`link: true` when the concatenation holds a
/// CJK ideograph). `match` is renamed `match_` (Rust keyword).
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

/// `match-readings-json` (`kanji.lisp:452`). `None` when `str` holds no
/// kanji-ish character or when [`match_readings`] cannot align;
/// otherwise the per-segment JSON list from [`process_match_json`].
pub async fn match_readings_json(
    ctx: &KaniranContext,
    str: &str,
    reading: &str,
) -> Result<Option<Vec<Value>>, sqlx::Error> {
    // kanji.lisp:453 ((ppcre:scan *kanji-regex* str))
    if !kanji_scanner()
        .is_match(str)
        .expect("scan over fixed *kanji-regex* pattern cannot fail")
    {
        return Ok(None);
    }
    // kanji.lisp:454-456 ((let ((match (match-readings str reading))) (when match (process-match-json match))))
    match match_readings(ctx, str, reading).await? {
        Some(match_) => Ok(Some(process_match_json(ctx, &match_).await?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// REPL fixtures (.103), 2026-05-09.
    #[test]
    fn get_original_reading_matches_repl_captures() {
        assert_eq!(get_original_reading("はる", false, None), "はる");
        assert_eq!(get_original_reading("ばる", true, None), "はる");
        assert_eq!(get_original_reading("はつ", false, Some("つ")), "はつ");
        assert_eq!(get_original_reading("ばっ", true, Some("つ")), "はつ");
    }

    async fn ctx() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103), 2026-05-24. Source words 人々/ひとびと,
    /// 学校/がっこう, 三日月/みかづき, 日本/にっぽん. 唖/あ pinned via
    /// the DB directly (null-grade row, `stat_common`=0 → `--.--%`).
    /// Covers link present/absent, rendaku/no-rendaku, gemination,
    /// stats present (with and without grade) vs absent, and all
    /// `get_original_reading` paths.
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

    /// REPL fixtures (.103), 2026-05-25. Covers kanji-then-text run,
    /// gemination, rendaku, all-irr coalesce, single-irr flush between
    /// normal kanji, the iteration mark 々 (no `link`), and a
    /// geminate-then-rendaku pair.
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

    /// REPL fixtures (.103), 2026-05-25. Covers the two `None`
    /// short-circuits (no kanji, no alignment) and positive results
    /// (irr fall-through, kanji+okurigana). Exhaustive shape coverage
    /// lives in [`process_match_json_fixtures`].
    #[tokio::test]
    async fn match_readings_json_fixtures() {
        let ctx = ctx().await;

        assert!(match_readings_json(&ctx, "みず", "みず").await.unwrap().is_none());
        assert!(match_readings_json(&ctx, "日本", "あ").await.unwrap().is_none());
        assert!(match_readings_json(&ctx, "今日", "").await.unwrap().is_none());

        let positives: &[(&str, &str, &str)] = &[
            (
                "水", "xyz",
                r#"[{"kanji":"水","reading":"xyz","type":"irr","link":true}]"#,
            ),
            (
                "見る", "みる",
                r#"[{"kanji":"見","reading":"み","type":"ja_kun","link":true,"stats":true,"sample":135,"total":173,"perc":"78.03%","grade":1},{"text":"る"}]"#,
            ),
        ];
        for (str, reading, expected) in positives {
            let result = match_readings_json(&ctx, str, reading)
                .await
                .unwrap()
                .expect("match aligns");
            let actual = serde_json::to_string(&result).unwrap();
            assert_eq!(actual.as_str(), *expected, "str={str} reading={reading}");
        }
    }
}
