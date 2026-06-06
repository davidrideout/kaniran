//! Port of `ichiran/dict:conj-info-json` (`dict.lisp:1697`).
//!
//! Wraps [`super::conj_info_json_star_::conj_info_json_star_`], keeping
//! only the `readok`-true entries and falling back to the full list
//! when none pass.

use serde_json::Value;

use crate::conn::kani_context::KaniranContext;

use super::conj_info_json_star_::conj_info_json_star_;
use super::filter_props::FilterPropsText;
use super::simple_text_class::WordConjugations;

// (jsown:val c "readok") as a generalized boolean — t is truthy, jsown's
// nil-rendering [] is falsy.
fn readok_truthy(entry: &Value) -> bool {
    match entry.get("readok") {
        Some(Value::Bool(readok)) => *readok,
        Some(Value::Array(readok)) => !readok.is_empty(),
        Some(Value::Null) | None => false,
        Some(_) => true,
    }
}

pub async fn conj_info_json(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: Option<&WordConjugations>,
    text: FilterPropsText<'_>,
    has_gloss: bool,
) -> Result<Vec<Value>, sqlx::Error> {
    // (apply 'conj-info-json* seq rest)
    let cij = conj_info_json_star_(ctx, seq, conjugations, text, has_gloss).await?;
    // (remove-if-not (lambda (c) (jsown:val c "readok")) cij)
    let fcij: Vec<Value> = cij.iter().filter(|entry| readok_truthy(entry)).cloned().collect();
    // (or fcij cij)
    Ok(if fcij.is_empty() { cij } else { fcij })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// REPL fixtures (.103, `(jsown:to-json (conj-info-json …))`), 2026-05-24.
    /// seq 10175587 (尽き果てる, ~ta) resolves the original reading with a
    /// matching surface (readok true → kept). With nil text every entry's
    /// readok is `[]`, so `remove-if-not` empties the filtered list and the
    /// `(or fcij cij)` fallback returns the unfiltered list unchanged.
    #[tokio::test]
    async fn readok_filter_and_fallback() {
        let ctx = ctx_from_env().await;
        let found = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":true}]"#;
        let unresolved = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":[]}]"#;

        let result = conj_info_json(&ctx, 10175587, None, FilterPropsText::One("つきはてた"), false)
            .await
            .unwrap();
        assert_eq!(json(&result), found, "resolved surface");

        let result = conj_info_json(&ctx, 10175587, None, FilterPropsText::None, false)
            .await
            .unwrap();
        assert_eq!(json(&result), unresolved, "nil text → fallback to cij");

        // has-gloss + unresolved reading drops the only entry in conj-info-json*,
        // so cij is empty and (or fcij cij) is the empty list.
        let result = conj_info_json(&ctx, 10175587, None, FilterPropsText::One("存在しない"), true)
            .await
            .unwrap();
        assert_eq!(json(&result), "[]", "has-gloss drop → empty");
    }

    /// REPL: `(conj-info-json 10670519 :conjugations nil :text "あくどくさせた"
    /// :has-gloss nil)`. The via-not-null entry keeps its recursive `via`
    /// payload (readok copied from the via's first element → true).
    #[tokio::test]
    async fn via_recursion_kept() {
        let ctx = ctx_from_env().await;
        let expected = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"via":[{"prop":[{"pos":"adj-i","type":"Causative"}],"reading":"悪どい 【あくどい】","gloss":[{"pos":"[adj-i]","gloss":"gaudy; showy; garish; loud"},{"pos":"[adj-i]","gloss":"crooked; vicious; wicked; nasty; unscrupulous; dishonest"}],"readok":true}],"readok":true}]"#;
        let result =
            conj_info_json(&ctx, 10670519, None, FilterPropsText::One("あくどくさせた"), false)
                .await
                .unwrap();
        assert_eq!(json(&result), expected);
    }

    /// REPL: `(conj-info-json 1156880 :conjugations nil :text "慰め")`. Both
    /// via-null entries resolve (readok true), so the filtered list equals
    /// the full two-entry list.
    #[tokio::test]
    async fn multi_entry_all_kept() {
        let ctx = ctx_from_env().await;
        let both = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":true}]"#;
        let result = conj_info_json(&ctx, 1156880, None, FilterPropsText::One("慰め"), false)
            .await
            .unwrap();
        assert_eq!(json(&result), both);
    }
}
