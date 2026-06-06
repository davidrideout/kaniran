//! Port of `ichiran/kanji:kanji-info-json` (`kanji.lisp:392`).
//!
//! Looks up the `kanji` row whose `text` equals `char` and returns its
//! [`Kanji::to_json`] object, or [`None`] when no row matches.

use serde_json::Value;

use super::kanji_dao::Kanji;
use super::to_json::to_json;
use crate::conn::kani_context::KaniranContext;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `jsown:to-json` of `(kanji-info-json …)`), 2026-05-25.
    ///
    /// Covers a present character (氷 — `freq`/`grade` set, mixed reading
    /// types, a `ja_kun` reading with okurigana), a character passed as the
    /// Lisp `#\光` (the one-character-string path the Rust `&str` already
    /// is), and a non-kanji argument (`"z"`) returning `None`.
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
}
