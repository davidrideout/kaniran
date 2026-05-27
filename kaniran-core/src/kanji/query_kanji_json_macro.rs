//! Port of `ichiran/kanji:query-kanji-json` (`kanji.lisp:458`).
//!
//! ```lisp
//! (defmacro query-kanji-json (var query &body extra-fields)
//!   (alexandria:with-gensyms (js)
//!     `(with-connection *connection*
//!        (mapcar (lambda (,var)
//!                  (let ((,js (to-json ,var)))
//!                    (jsown:extend-js ,js ,@extra-fields)))
//!                (query-dao 'kanji ,query)))))
//! ```
//!
//! Runs `query` as a `kanji`-DAO query, maps each row through
//! [`to_json`], and extends each resulting object with the caller's
//! extra fields. The `&body extra-fields` (unevaluated `(key value)`
//! forms over the bound row) becomes the `extra_fields` closure
//! returning the per-row `(key, value)` pairs; `query` becomes the SQL
//! `&str` passed to `query-dao`; the gensym row binding `var` becomes
//! the closure parameter.
//!
//! Diverges from the upstream lambda list `(var query &body extra-fields)`
//! by taking `&KaniranContext` for the database handle, replacing the
//! upstream dynamic `*connection*` per [`crate::conn::kani_context`].

use serde_json::Value;

use super::kanji_dao::Kanji;
use super::to_json::to_json;
use crate::conn::kani_context::KaniranContext;

pub async fn query_kanji_json(
    ctx: &KaniranContext,
    query: &str,
    extra_fields: impl Fn(&Kanji) -> Vec<(String, Value)>,
) -> Result<Vec<Value>, sqlx::Error> {
    let mut result = Vec::new();
    // (query-dao 'kanji query)
    let rows: Vec<Kanji> = sqlx::query_as(query).fetch_all(&ctx.pool).await?;
    // (mapcar (lambda (var) (let ((js (to-json var))) (jsown:extend-js js …))) …)
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

    /// REPL fixtures (.103, `jsown:to-json` of a `query-kanji-json`
    /// invocation), 2026-05-26. 檸 has the smallest `to-json` shape
    /// (no freq/grade, two readings, one meaning); the two extra fields
    /// read the bound row's `text` and `id`, appended after the base
    /// object in invocation order.
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

    /// A multi-row query maps each row and applies the extra field to every
    /// object; an empty result set yields an empty list.
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
