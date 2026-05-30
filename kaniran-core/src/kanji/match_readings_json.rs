//! Port of `ichiran/kanji:match-readings-json` (`kanji.lisp:452`).
//!
//! ```lisp
//! (defun match-readings-json (str reading)
//!   (and (ppcre:scan *kanji-regex* str)
//!        (let ((match (match-readings str reading)))
//!          (when match
//!            (process-match-json match)))))
//! ```
//!
//! Returns [`None`] when `str` holds no kanji-ish character, or when
//! [`super::match_readings`] cannot align `reading` to it; otherwise the
//! per-segment JSON list from [`super::process_match_json`]. The
//! upstream `(when match …)` guard is subsumed by `match_readings`'s
//! [`Option`] (its `:none`/NIL signal is already `None`).
//!
//! Diverges from the upstream lambda list `(str reading)` only by taking
//! `&KaniranContext` for the database handle (via
//! [`super::match_readings`] / [`super::process_match_json`]), replacing
//! the upstream dynamic `*connection*` per [`crate::conn::kani_context`].

use std::sync::OnceLock;

use fancy_regex::Regex;
use serde_json::Value;

use super::match_readings::match_readings;
use super::process_match_json::process_match_json;
use crate::characters::char_classes::KANJI_REGEX;
use crate::conn::kani_context::KaniranContext;

static KANJI_SCANNER: OnceLock<Regex> = OnceLock::new();

fn kanji_scanner() -> &'static Regex {
    KANJI_SCANNER.get_or_init(|| Regex::new(KANJI_REGEX).expect("*kanji-regex* must compile"))
}

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

    async fn ctx() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `jsown:to-json` of `(match-readings-json str reading)`),
    /// 2026-05-25.
    ///
    /// Covers the two `None` short-circuits — no kanji in `str`
    /// (みず/みず) and a kanji `str` that `match-readings` cannot align
    /// (日本/あ, 今日/"") — plus positive results: an irr fall-through
    /// when the reading matches no candidate (水/xyz → 水 irr) and a
    /// kanji-plus-okurigana word (見る/みる). The exhaustive JSON-shape
    /// coverage lives in [`super::super::process_match_json`]'s tests.
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
