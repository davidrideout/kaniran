//! Per-kanji-row JSON rendering. From `kanji.lisp:349-395, 458-470`.

use serde_json::{Map, Value};

use super::dao::{Kanji, Meaning, Reading};
use crate::conn::kani_context::KaniranContext;
use crate::core::_star_hepburn_basic_star_::hepburn_basic;
use crate::core::generic_romanization_class::RomanizationMethod;
use crate::core::romanize_word::romanize_word;

/// `calculate-perc` (`kanji.lisp:349`). `sample/total` as a fixed-width
/// percentage with two fractional digits. `total = 0` yields `--.--%`,
/// mirroring upstream's divide-by-zero guard. The `~,2,,,'0F%` format
/// directive rounds half-to-even, matching Rust's default `{:.2}`.
pub fn calculate_perc(sample: i32, total: i32) -> String {
    if total == 0 {
        "--.--%".to_string()
    } else {
        format!("{:.2}%", 100.0 * sample as f64 / total as f64)
    }
}

/// `reading-info-json` (`kanji.lisp:354`). Per-reading object with
/// `{text, rtext, type, okuri, sample, perc}` and optional
/// `prefixp`/`suffixp` flags. `rtext` is the hepburn-basic
/// romanization. `okuri` is the distinct text column of the
/// `okurigana` rows for this reading.
pub async fn reading_info_json(
    ctx: &KaniranContext,
    reading: &Reading,
    total: i32,
) -> Result<Value, sqlx::Error> {
    let mut js = Map::new();
    js.insert("text".to_owned(), Value::String(reading.text.clone()));
    // kanji.lisp:358 ((romanize-word (text reading) :method *hepburn-basic* :original-spelling ""))
    js.insert(
        "rtext".to_owned(),
        Value::String(romanize_word(
            &reading.text,
            RomanizationMethod::GenericHepburn(hepburn_basic()),
            Some(""),
            true,
        )),
    );
    js.insert(
        "type".to_owned(),
        Value::String(reading.reading_type.clone()),
    );
    // kanji.lisp:360 ((query (:select 'text :distinct :from 'okurigana :where (:= 'reading-id (id reading))) :column))
    let okuri: Vec<String> = sqlx::query_scalar("SELECT DISTINCT text FROM okurigana WHERE reading_id = $1")
        .bind(reading.id)
        .fetch_all(&ctx.pool)
        .await?;
    js.insert(
        "okuri".to_owned(),
        Value::Array(okuri.into_iter().map(Value::String).collect()),
    );
    js.insert(
        "sample".to_owned(),
        Value::Number(reading.stat_common.into()),
    );
    js.insert(
        "perc".to_owned(),
        Value::String(calculate_perc(reading.stat_common, total)),
    );
    if reading.prefixp {
        js.insert("prefixp".to_owned(), Value::Bool(true));
    }
    if reading.suffixp {
        js.insert("suffixp".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(js))
}

/// `to-json` (`kanji.lisp:369`), the sole method of the `to-json`
/// generic. Per-kanji object with metadata, readings array, meanings
/// array, and always-emitted `freq`/`grade` (a SQL NULL reads as
/// `:null`, truthy in Lisp, so the `(when …)` guards always fire and a
/// NULL column becomes JSON null).
pub async fn to_json(ctx: &KaniranContext, kanji: &Kanji) -> Result<Value, sqlx::Error> {
    let total = kanji.stat_common;
    let mut js = Map::new();
    js.insert("text".to_owned(), Value::String(kanji.text.clone()));
    js.insert("rc".to_owned(), Value::Number(kanji.radical_c.into()));
    js.insert("rn".to_owned(), Value::Number(kanji.radical_n.into()));
    js.insert("strokes".to_owned(), Value::Number(kanji.strokes.into()));
    js.insert("total".to_owned(), Value::Number(kanji.stat_common.into()));
    js.insert("irr".to_owned(), Value::Number(kanji.stat_irregular.into()));
    js.insert(
        "irr_perc".to_owned(),
        Value::String(calculate_perc(kanji.stat_irregular, total)),
    );
    // kanji.lisp:379-383 ((select-dao 'reading (:and (:= 'kanji-id (id kanji)) (:not (:= 'type "ja_na"))) (:desc 'type) (:desc 'stat-common)))
    let readings: Vec<Reading> = sqlx::query_as(
        "SELECT * FROM reading WHERE kanji_id = $1 AND NOT (type = 'ja_na') \
         ORDER BY type DESC, stat_common DESC",
    )
    .bind(kanji.id)
    .fetch_all(&ctx.pool)
    .await?;
    let mut readings_json = Vec::with_capacity(readings.len());
    for reading in &readings {
        readings_json.push(reading_info_json(ctx, reading, total).await?);
    }
    js.insert("readings".to_owned(), Value::Array(readings_json));
    // kanji.lisp:384 ((mapcar 'text (select-dao 'meaning (:= 'kanji-id (id kanji)) 'id)))
    let meanings: Vec<Meaning> =
        sqlx::query_as("SELECT * FROM meaning WHERE kanji_id = $1 ORDER BY id")
            .bind(kanji.id)
            .fetch_all(&ctx.pool)
            .await?;
    js.insert(
        "meanings".to_owned(),
        Value::Array(meanings.iter().map(|m| Value::String(m.text.clone())).collect()),
    );
    js.insert(
        "freq".to_owned(),
        match kanji.freq {
            Some(freq) => Value::Number(freq.into()),
            None => Value::Null,
        },
    );
    js.insert(
        "grade".to_owned(),
        match kanji.grade {
            Some(grade) => Value::Number(grade.into()),
            None => Value::Null,
        },
    );
    Ok(Value::Object(js))
}

/// `kanji-info-json` (`kanji.lisp:392`). Looks up the `kanji` row whose
/// `text` equals `char` and returns its [`to_json`] object, or `None`
/// when no row matches.
pub async fn kanji_info_json(
    ctx: &KaniranContext,
    char: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let str = char;
    // kanji.lisp:395 ((car (select-dao 'kanji (:= 'text str))))
    let kanji: Option<Kanji> = sqlx::query_as("SELECT * FROM kanji WHERE text = $1")
        .bind(str)
        .fetch_all(&ctx.pool)
        .await?
        .into_iter()
        .next();
    match kanji {
        Some(kanji) => Ok(Some(to_json(ctx, &kanji).await?)),
        None => Ok(None),
    }
}

/// `query-kanji-json` (`kanji.lisp:458`). Runs `query` as a kanji-DAO
/// query, maps each row through [`to_json`], and extends each object
/// with the caller's extra fields. The upstream `&body extra-fields`
/// (unevaluated `(key value)` forms over the bound row) becomes
/// `extra_fields: impl Fn(&Kanji) -> Vec<(String, Value)>`.
pub async fn query_kanji_json(
    ctx: &KaniranContext,
    query: &str,
    extra_fields: impl Fn(&Kanji) -> Vec<(String, Value)>,
) -> Result<Vec<Value>, sqlx::Error> {
    let mut result = Vec::new();
    let rows: Vec<Kanji> = sqlx::query_as(query).fetch_all(&ctx.pool).await?;
    for var in &rows {
        let mut js = to_json(ctx, var).await?;
        if let Value::Object(map) = &mut js {
            for (key, value) in extra_fields(var) {
                map.insert(key, value);
            }
        }
        result.push(js);
    }
    Ok(result)
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

    /// REPL fixtures (.103), 2026-05-09.
    #[test]
    fn calculate_perc_matches_repl_captures() {
        assert_eq!(calculate_perc(50, 100), "50.00%");
        assert_eq!(calculate_perc(1, 1000), "0.10%");
        assert_eq!(calculate_perc(0, 0), "--.--%");
        assert_eq!(calculate_perc(33, 100), "33.00%");
        assert_eq!(calculate_perc(1, 3), "33.33%");
        assert_eq!(calculate_perc(1, 7), "14.29%");
        assert_eq!(calculate_perc(5, 100), "5.00%");
        assert_eq!(calculate_perc(100, 100), "100.00%");
        assert_eq!(calculate_perc(3, 7), "42.86%");
    }

    async fn reading(ctx: &KaniranContext, id: i32) -> Reading {
        sqlx::query_as::<_, Reading>("SELECT * FROM reading WHERE id = $1")
            .bind(id)
            .fetch_one(&ctx.pool)
            .await
            .expect("reading row exists")
    }

    /// REPL fixtures (.103), 2026-05-24. Covers multi/single/empty
    /// okurigana arrays; prefixp/suffixp combinations; `--.--%` vs
    /// `0.00%` vs nonzero `perc`; generic-hepburn `rtext`.
    #[tokio::test]
    async fn reading_info_json_fixtures() {
        let ctx = ctx().await;
        let cases: &[(i32, i32, &str)] = &[
            (
                3329, 345,
                r#"{"text":"む","rtext":"mu","type":"ja_kun","okuri":["かう","い","ける","き","け","く","かい","こう"],"sample":37,"perc":"10.72%","prefixp":true,"suffixp":true}"#,
            ),
            (
                5014, 200,
                r#"{"text":"で","rtext":"de","type":"ja_kun","okuri":["る"],"sample":78,"perc":"39.00%","suffixp":true}"#,
            ),
            (
                5702, 100,
                r#"{"text":"もう","rtext":"mou","type":"ja_kun","okuri":["し","す"],"sample":22,"perc":"22.00%","prefixp":true}"#,
            ),
            (
                397, 50,
                r#"{"text":"び","rtext":"bi","type":"ja_kun","okuri":["き"],"sample":15,"perc":"30.00%","suffixp":true}"#,
            ),
            (
                575, 0,
                r#"{"text":"え","rtext":"e","type":"ja_na","okuri":[],"sample":0,"perc":"--.--%"}"#,
            ),
            (
                575, 120,
                r#"{"text":"え","rtext":"e","type":"ja_na","okuri":[],"sample":0,"perc":"0.00%"}"#,
            ),
            (
                315, 314,
                r#"{"text":"いち","rtext":"ichi","type":"ja_on","okuri":[],"sample":314,"perc":"100.00%"}"#,
            ),
        ];
        for (id, total, expected) in cases {
            let reading = reading(&ctx, *id).await;
            let js = reading_info_json(&ctx, &reading, *total).await.unwrap();
            let actual = serde_json::to_string(&js).unwrap();
            assert_eq!(actual.as_str(), *expected, "id={id} total={total}");
        }
    }

    async fn kanji(ctx: &KaniranContext, text: &str) -> Kanji {
        sqlx::query_as::<_, Kanji>("SELECT * FROM kanji WHERE text = $1")
            .bind(text)
            .fetch_one(&ctx.pool)
            .await
            .expect("kanji row exists")
    }

    /// REPL fixtures (.103), 2026-05-25. Covers always-emit
    /// `freq`/`grade` (NULL → JSON null), `irr_perc` with `total=0` →
    /// `--.--%` and nonzero, `type`-desc/`sample`-desc reading order,
    /// `suffixp` readings, `ja_kun` with okurigana.
    #[tokio::test]
    async fn to_json_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, &str)] = &[
            (
                "人",
                r#"{"text":"人","rc":9,"rn":9,"strokes":2,"total":345,"irr":5,"irr_perc":"1.45%","readings":[{"text":"じん","rtext":"jin","type":"ja_on","okuri":[],"sample":174,"perc":"50.43%"},{"text":"にん","rtext":"nin","type":"ja_on","okuri":[],"sample":96,"perc":"27.83%"},{"text":"ひと","rtext":"hito","type":"ja_kun","okuri":[],"sample":47,"perc":"13.62%"},{"text":"り","rtext":"ri","type":"ja_kun","okuri":[],"sample":15,"perc":"4.35%","suffixp":true},{"text":"と","rtext":"to","type":"ja_kun","okuri":[],"sample":8,"perc":"2.32%","suffixp":true}],"meanings":["person"],"freq":5,"grade":1}"#,
            ),
            (
                "薔",
                r#"{"text":"薔","rc":140,"rn":140,"strokes":16,"total":0,"irr":0,"irr_perc":"--.--%","readings":[{"text":"ば","rtext":"ba","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"しょう","rtext":"shou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"しょく","rtext":"shoku","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"そう","rtext":"sou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"みずたで","rtext":"mizutade","type":"ja_kun","okuri":[],"sample":0,"perc":"--.--%"}],"meanings":["a kind of grass"],"freq":2356,"grade":null}"#,
            ),
            (
                "鬱",
                r#"{"text":"鬱","rc":192,"rn":75,"strokes":29,"total":3,"irr":0,"irr_perc":"0.00%","readings":[{"text":"うつ","rtext":"utsu","type":"ja_on","okuri":[],"sample":2,"perc":"66.67%"},{"text":"うっ","rtext":"u","type":"ja_kun","okuri":["する"],"sample":1,"perc":"33.33%"},{"text":"ふさ","rtext":"fusa","type":"ja_kun","okuri":["ぐ"],"sample":0,"perc":"0.00%"},{"text":"しげ","rtext":"shige","type":"ja_kun","okuri":["る"],"sample":0,"perc":"0.00%"}],"meanings":["gloom","depression","melancholy","luxuriant"],"freq":null,"grade":8}"#,
            ),
            (
                "檸",
                r#"{"text":"檸","rc":75,"rn":75,"strokes":18,"total":0,"irr":0,"irr_perc":"--.--%","readings":[{"text":"ねい","rtext":"nei","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"どう","rtext":"dou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"}],"meanings":["lemon tree"],"freq":null,"grade":null}"#,
            ),
        ];
        for (text, expected) in cases {
            let kanji = kanji(&ctx, text).await;
            let js = to_json(&ctx, &kanji).await.unwrap();
            let actual = serde_json::to_string(&js).unwrap();
            assert_eq!(actual.as_str(), *expected, "text={text}");
        }
    }

    /// REPL fixtures (.103), 2026-05-25. Covers a present character
    /// (氷), a one-char-string path (光), and a non-kanji argument (`"z"`)
    /// returning `None`.
    #[tokio::test]
    async fn kanji_info_json_fixtures() {
        let ctx = ctx().await;
        assert_eq!(
            serde_json::to_string(&kanji_info_json(&ctx, "氷").await.unwrap().unwrap()).unwrap(),
            r#"{"text":"氷","rc":85,"rn":3,"strokes":5,"total":14,"irr":1,"irr_perc":"7.14%","readings":[{"text":"ひょう","rtext":"hyou","type":"ja_on","okuri":[],"sample":9,"perc":"64.29%"},{"text":"こおり","rtext":"koori","type":"ja_kun","okuri":[],"sample":3,"perc":"21.43%"},{"text":"ひ","rtext":"hi","type":"ja_kun","okuri":[],"sample":1,"perc":"7.14%"},{"text":"こお","rtext":"koo","type":"ja_kun","okuri":["る"],"sample":0,"perc":"0.00%"}],"meanings":["icicle","ice","hail","freeze","congeal"],"freq":1450,"grade":3}"#,
        );
        assert_eq!(
            serde_json::to_string(&kanji_info_json(&ctx, "光").await.unwrap().unwrap()).unwrap(),
            r#"{"text":"光","rc":10,"rn":42,"strokes":6,"total":40,"irr":0,"irr_perc":"0.00%","readings":[{"text":"こう","rtext":"kou","type":"ja_on","okuri":[],"sample":36,"perc":"90.00%"},{"text":"ひか","rtext":"hika","type":"ja_kun","okuri":["る"],"sample":3,"perc":"7.50%"},{"text":"ひかり","rtext":"hikari","type":"ja_kun","okuri":[],"sample":1,"perc":"2.50%"}],"meanings":["ray","light"],"freq":527,"grade":2}"#,
        );
        assert!(kanji_info_json(&ctx, "z").await.unwrap().is_none());
    }

    /// REPL fixtures (.103), 2026-05-26.
    #[tokio::test]
    async fn query_kanji_json_single_row_extra_fields() {
        let ctx = ctx().await;
        let result = query_kanji_json(
            &ctx,
            "select * from kanji where text = '檸'",
            |var| {
                vec![
                    ("custom".to_owned(), Value::String(var.text.clone())),
                    ("rid".to_owned(), Value::Number(var.id.into())),
                ]
            },
        )
        .await
        .unwrap();
        let expected = r#"[{"text":"檸","rc":75,"rn":75,"strokes":18,"total":0,"irr":0,"irr_perc":"--.--%","readings":[{"text":"ねい","rtext":"nei","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"どう","rtext":"dou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"}],"meanings":["lemon tree"],"freq":null,"grade":null,"custom":"檸","rid":4193}]"#;
        assert_eq!(serde_json::to_string(&result).unwrap().as_str(), expected);
    }

    #[tokio::test]
    async fn query_kanji_json_multi_and_empty() {
        let ctx = ctx().await;
        let multi = query_kanji_json(
            &ctx,
            "select * from kanji where text in ('檸','薔') order by text",
            |_var| vec![("mark".to_owned(), Value::Bool(true))],
        )
        .await
        .unwrap();
        assert_eq!(multi.len(), 2);
        for obj in &multi {
            assert_eq!(obj["mark"], Value::Bool(true));
            assert!(obj.get("text").is_some(), "to-json fields present");
        }

        let empty = query_kanji_json(&ctx, "select * from kanji where text = 'ZZZ'", |_var| vec![])
            .await
            .unwrap();
        assert!(empty.is_empty());
    }
}
