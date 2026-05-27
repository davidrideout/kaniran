//! Port of `ichiran:romaji-suggest` (`deromanize.lisp:95`).
//!
//! ```lisp
//! (defun romaji-suggest (s)
//!   (multiple-value-bind (canon pattern) (romaji-kana s)
//!     (when pattern
//!       (multiple-value-bind (pkanji pkana) (find-kanji-for-pattern pattern)
//!         (let ((hiragana (remove-duplicates (cons canon pkana) :test 'equal :from-end t)))
//!           (jsown:new-js
//!             ("hiragana" hiragana)
//!             ("katakana" (mapcar 'as-katakana hiragana))
//!             ("kanji" pkanji)))))))
//! ```
//!
//! Deromanizes `s`, looks up the kanji and kana matching the resulting
//! kana pattern, and returns a `{"hiragana", "katakana", "kanji"}`
//! object. `hiragana` is the canonical kana followed by the pattern's
//! kana readings with duplicates removed (first kept); `katakana` is
//! each of those as katakana; `kanji` is the matched kanji. Returns
//! `None` when `s` does not deromanize (upstream returns `nil`).
//!
//! Diverges from the upstream lambda list `(s)` by taking
//! `&KaniranContext` for the database handle (via
//! [`find_kanji_for_pattern`]), replacing the upstream dynamic
//! `*connection*` per [`crate::conn::kani_context`].

use std::collections::HashSet;

use serde_json::{Map, Value};

use super::romaji_kana::romaji_kana;
use crate::characters::as_katakana::as_katakana;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_kanji_for_pattern::find_kanji_for_pattern;

pub async fn romaji_suggest(ctx: &KaniranContext, s: &str) -> Result<Option<Value>, sqlx::Error> {
    // (multiple-value-bind (canon pattern) (romaji-kana s) (when pattern …))
    let Some((canon, pattern)) = romaji_kana(s) else {
        return Ok(None);
    };
    // (multiple-value-bind (pkanji pkana) (find-kanji-for-pattern pattern) …)
    let (pkanji, pkana) = find_kanji_for_pattern(ctx, &pattern).await?;
    // (remove-duplicates (cons canon pkana) :test 'equal :from-end t)
    let mut hiragana_src = Vec::with_capacity(pkana.len() + 1);
    hiragana_src.push(canon);
    hiragana_src.extend(pkana);
    let hiragana = remove_duplicates_from_end(hiragana_src);
    // (jsown:new-js ("hiragana" …) ("katakana" (mapcar 'as-katakana …)) ("kanji" pkanji))
    let mut js = Map::new();
    js.insert(
        "hiragana".to_owned(),
        Value::Array(hiragana.iter().map(|h| Value::String(h.clone())).collect()),
    );
    js.insert(
        "katakana".to_owned(),
        Value::Array(hiragana.iter().map(|h| Value::String(as_katakana(h))).collect()),
    );
    js.insert(
        "kanji".to_owned(),
        Value::Array(pkanji.into_iter().map(Value::String).collect()),
    );
    Ok(Some(Value::Object(js)))
}

// `(remove-duplicates … :test 'equal :from-end t)` — keep the first
// occurrence of each value, preserving order.
fn remove_duplicates_from_end(items: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    items.into_iter().filter(|item| seen.insert(item.clone())).collect()
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

    /// REPL fixtures (.103, `jsown:to-json` of `(romaji-suggest s)`),
    /// 2026-05-26. Full-JSON comparison on words whose matched-kanji order
    /// is deterministic: `neko`/`tegami`/`toukyou` (single kanji) and `hon`
    /// (two kanji, stable across repeated runs). Exercises the object
    /// shape, the canonical-kana hiragana entry, and `as-katakana` mapping.
    #[tokio::test]
    async fn romaji_suggest_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, &str)] = &[
            ("neko", r#"{"hiragana":["ねこ"],"katakana":["ネコ"],"kanji":["猫"]}"#),
            ("tegami", r#"{"hiragana":["てがみ"],"katakana":["テガミ"],"kanji":["手紙"]}"#),
            ("toukyou", r#"{"hiragana":["とうきょう"],"katakana":["トウキョウ"],"kanji":["東京"]}"#),
            ("hon", r#"{"hiragana":["ほん"],"katakana":["ホン"],"kanji":["本","品"]}"#),
        ];
        for (s, expected) in cases {
            let js = romaji_suggest(&ctx, s).await.unwrap().expect("deromanizes");
            assert_eq!(serde_json::to_string(&js).unwrap().as_str(), *expected, "s={s}");
        }
    }

    /// `inu` deromanizes to canonical いぬ but its pattern also matches the
    /// alternative kana reading いんう, so hiragana carries both (canonical
    /// first, then the deduped pattern readings) and katakana maps each.
    /// The matched-kanji order is DB-scan-dependent for tied `common`, so
    /// only the deterministic hiragana/katakana fields are pinned here.
    #[tokio::test]
    async fn romaji_suggest_multi_hiragana() {
        let ctx = ctx().await;
        let js = romaji_suggest(&ctx, "inu").await.unwrap().expect("deromanizes");
        assert_eq!(js["hiragana"], serde_json::json!(["いぬ", "いんう"]));
        assert_eq!(js["katakana"], serde_json::json!(["イヌ", "インウ"]));
    }

    /// `xyz` does not deromanize → romaji-kana returns nil → None.
    #[tokio::test]
    async fn romaji_suggest_no_parse() {
        let ctx = ctx().await;
        assert!(romaji_suggest(&ctx, "xyz").await.unwrap().is_none());
    }
}
