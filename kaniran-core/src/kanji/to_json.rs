//! Port of `ichiran/kanji:to-json` (`kanji.lisp:369`), the sole method
//! of the `to-json` generic (`kanji.lisp:7`).
//!
//! ```lisp
//! (defmethod to-json ((kanji kanji) &key)
//!   (let* ((total (stat-common kanji))
//!          (js (jsown:new-js ("text" (text kanji)) ("rc" (radical-c kanji))
//!                 ("rn" (radical-n kanji)) ("strokes" (strokes kanji))
//!                 ("total" (stat-common kanji)) ("irr" (stat-irregular kanji))
//!                 ("irr_perc" (calculate-perc (stat-irregular kanji) total))
//!                 ("readings" (mapcar (lambda (r) (reading-info-json r total))
//!                               (select-dao 'reading (:and (:= 'kanji-id (id kanji))
//!                                  (:not (:= 'type "ja_na"))) (:desc 'type) (:desc 'stat-common))))
//!                 ("meanings" (mapcar 'text (select-dao 'meaning (:= 'kanji-id (id kanji)) 'id))))))
//!     (when (freq kanji) (jsown:extend-js js ("freq" (freq kanji))))
//!     (when (grade kanji) (jsown:extend-js js ("grade" (grade kanji))))
//!     js))
//! ```
//!
//! Returns a [`serde_json::Value`] object (insertion order via the
//! crate's `preserve_order` feature). The generic has only this one
//! method, so it is ported as a single free function over [`Kanji`]
//! rather than an enum dispatcher.
//!
//! Diverges from the upstream lambda list `(kanji &key)` only by taking
//! `&KaniranContext` for the database handle (the readings/meanings
//! queries), replacing the upstream dynamic `*connection*` per
//! [`crate::conn::kani_context`].

use serde_json::{Map, Value};

use super::calculate_perc::calculate_perc;
use super::kanji_dao::Kanji;
use super::meaning_dao::Meaning;
use super::reading_dao::Reading;
use super::reading_info_json::reading_info_json;
use crate::conn::kani_context::KaniranContext;

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
    // kanji.lisp:386-389 ((when (freq kanji) ...) / (when (grade kanji) ...)) — the slot reads
    // :null for a SQL NULL, which is truthy in Lisp, so both keys are always emitted; a NULL
    // column becomes JSON null.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    async fn kanji(ctx: &KaniranContext, text: &str) -> Kanji {
        sqlx::query_as::<_, Kanji>("SELECT * FROM kanji WHERE text = $1")
            .bind(text)
            .fetch_one(&ctx.pool)
            .await
            .expect("kanji row exists")
    }

    /// REPL fixtures (.103, `jsown:to-json` of `(to-json kanji)`), 2026-05-25.
    ///
    /// Covers the always-emit `freq`/`grade` behaviour (a SQL NULL reads as
    /// `:null`, truthy in Lisp, so the `(when …)` guards always fire and a
    /// missing column becomes JSON null): 人 (both present), 薔 (grade null,
    /// freq present), 鬱 (freq null, grade present), 檸 (both null). Also
    /// exercises `irr_perc` with `total=0` → `--.--%` (薔, 檸) vs nonzero
    /// (人, 鬱); the `type`-desc/`sample`-desc reading order; `suffixp`
    /// readings (人's り/と); and `ja_kun` readings carrying okurigana (鬱).
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
}
