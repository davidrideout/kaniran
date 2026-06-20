//! v1 JSON — ichiran's nested positional format (legacy `-f`), faithful to
//! the original `jsown` output.

use std::error::Error;

use serde_json::{Number, Value};

use crate::conn::kani_context::KaniranContext;
use crate::core::kani_romanize_method::KaniRomanizeMethod;
use crate::core::romanize::RomanizeStarSegment;
use crate::dict::word_info_str::word_info_gloss_json;

/// Render `input` as v1 nested JSON.
pub(super) fn render(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    limit: usize,
) -> Result<String, Box<dyn Error>> {
    let result = super::segment(ctx, input, method, limit)?;
    Ok(serde_json::to_string(&to_json(ctx, &result)?)?)
}

// cli.lisp:41 (defmethod jsown:to-json ((word-info word-info))) +
// cli.lisp:87 (jsown:to-json result): jsown over the romanize* nested list.
// A misc split is its bare string; a word split is the list of
// (word-list score) pairs; each word is the triple (romanized word prop).
// The word-info renders via word-info-gloss-json (the cli.lisp method) and
// the prop is the default (constantly nil) wordprop-fn's nil, which jsown
// renders as [].
fn to_json(
    ctx: &KaniranContext,
    result: &[RomanizeStarSegment<()>],
) -> Result<Value, crate::conn::KaniDbError> {
    let mut parts = Vec::with_capacity(result.len());
    for segment in result {
        match segment {
            RomanizeStarSegment::Misc(split_text) => parts.push(Value::String(split_text.clone())),
            RomanizeStarSegment::Word(alternatives) => {
                let mut pairs = Vec::with_capacity(alternatives.len());
                for (word_list, score) in alternatives {
                    let mut words = Vec::with_capacity(word_list.len());
                    for (romanized, word, _prop) in word_list {
                        let gloss = word_info_gloss_json(ctx, word, false)?;
                        // Build the triple directly so `gloss` is moved, not
                        // re-serialized through json!'s to_value (which deep-
                        // copies the whole tree through the serde machinery).
                        words.push(Value::Array(vec![
                            Value::String(romanized.clone()),
                            gloss,
                            Value::Array(Vec::new()),
                        ]));
                    }
                    pairs.push(Value::Array(vec![
                        Value::Array(words),
                        Value::Number(Number::from(*score)),
                    ]));
                }
                parts.push(Value::Array(pairs));
            }
        }
    }
    Ok(Value::Array(parts))
}

#[cfg(test)]
mod tests {
    //! Ground truth from `(princ (jsown:to-json (romanize* input :limit 1)))`
    use super::*;
    use crate::core::methods::{hepburn_traditional, RomanizationMethod};
    use crate::core::romanize::romanize_star_;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn method() -> KaniRomanizeMethod<'static> {
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
    }

    #[test]
    fn full_json_matches_cli() {
        let ctx = ctx();
        // (input, limit, expected jsown:to-json output)
        let cases: &[(&str, usize, &str)] = &[
            // single word split, one alternative.
            (
                "世界",
                1,
                r#"[[[[["sekai",{"reading":"世界 【せかい】","text":"世界","kana":"せかい","score":325,"seq":1373860,"gloss":[{"pos":"[n]","gloss":"the world; society; the universe"},{"pos":"[n]","gloss":"sphere; circle; world"},{"pos":"[adj-no]","gloss":"world-renowned; world-famous"},{"pos":"[n]","gloss":"realm governed by one Buddha; space","field":"{Buddh}","info":"original meaning"}],"conj":[]},[]]],325]]]"#,
            ),
            // misc + word + misc: latin prefix, word split, "! " trailer.
            (
                "Hello 世界！",
                1,
                r#"["Hello ",[[[["sekai",{"reading":"世界 【せかい】","text":"世界","kana":"せかい","score":325,"seq":1373860,"gloss":[{"pos":"[n]","gloss":"the world; society; the universe"},{"pos":"[n]","gloss":"sphere; circle; world"},{"pos":"[adj-no]","gloss":"world-renowned; world-famous"},{"pos":"[n]","gloss":"realm governed by one Buddha; space","field":"{Buddh}","info":"original meaning"}],"conj":[]},[]]],325]],"! "]"#,
            ),
            // another single word split (counter-adjacent noun).
            (
                "三人",
                1,
                r#"[[[[["sannin",{"reading":"三人 【さんにん】","text":"三人","kana":"さんにん","score":325,"seq":1301000,"gloss":[{"pos":"[n]","gloss":"three people"}],"conj":[]},[]]],325]]]"#,
            ),
        ];
        for (input, limit, expected) in cases {
            let result = romanize_star_(&ctx, input, method(), Some(*limit), |_, _| ()).unwrap();
            let json = to_json(&ctx, &result).unwrap();
            assert_eq!(
                serde_json::to_string(&json).unwrap(),
                *expected,
                "input={input}"
            );
        }
    }
}
