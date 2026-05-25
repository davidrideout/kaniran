//! Port of `ichiran/kanji:reading-info-json` (`kanji.lisp:354`).
//!
//! ```lisp
//! (defun reading-info-json (reading total)
//!   (with-connection *connection*
//!     (let ((js (jsown:new-js
//!                 ("text" (text reading))
//!                 ("rtext" (romanize-word (text reading) :method *hepburn-basic* :original-spelling ""))
//!                 ("type" (reading-type reading))
//!                 ("okuri" (query (:select 'text :distinct :from 'okurigana :where (:= 'reading-id (id reading))) :column))
//!                 ("sample" (stat-common reading))
//!                 ("perc" (calculate-perc (stat-common reading) total)))))
//!       (when (prefixp reading)
//!         (jsown:extend-js js ("prefixp" t)))
//!       (when (suffixp reading)
//!         (jsown:extend-js js ("suffixp" t)))
//!       js)))
//! ```
//!
//! Returns a [`serde_json::Value`] object (insertion order via the crate's
//! `preserve_order` feature). The `:column` query yields the okurigana
//! `text` column as a JSON array (empty list renders `[]`).
//!
//! Diverges from the upstream lambda list `(reading total)` by taking
//! `&KaniranContext` first for the database handle (the okurigana query),
//! replacing the upstream dynamic `*connection*` per
//! [`crate::conn::kani_context`].

use serde_json::{Map, Value};

use super::calculate_perc::calculate_perc;
use super::reading_dao::Reading;
use crate::conn::kani_context::KaniranContext;
use crate::core::_star_hepburn_basic_star_::hepburn_basic;
use crate::core::generic_romanization_class::RomanizationMethod;
use crate::core::romanize_word::romanize_word;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    async fn reading(ctx: &KaniranContext, id: i32) -> Reading {
        sqlx::query_as::<_, Reading>("SELECT * FROM reading WHERE id = $1")
            .bind(id)
            .fetch_one(&ctx.pool)
            .await
            .expect("reading row exists")
    }

    /// REPL fixtures (.103, `jsown:to-json` of `reading-info-json`), 2026-05-24.
    /// Reading rows pinned by id (kanjidic2 load, identical on .103 and local).
    ///
    /// Covers: multi-element okurigana array (3329 む, 8 entries) vs single
    /// (5014 で, 397 び) vs empty `[]` (575 え, 315 いち); prefixp+suffixp both
    /// set (3329), prefixp only (5702 もう), suffixp only (5014/397), neither
    /// (575/315); `calculate-perc` with `total=0` → `--.--%` (575) vs `sample=0`
    /// total>0 → `0.00%` (575) vs nonzero (315 → `100.00%`); generic-hepburn
    /// `rtext` keeping long vowels (もう→`mou`, not `mō`).
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
}
