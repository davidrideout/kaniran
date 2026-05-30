//! Port of `ichiran/dict:conj-prop-json` (`dict.lisp:285`).
//!
//! ```lisp
//! (defun conj-prop-json (obj)
//!   (let ((js (jsown:new-js
//!               ("pos" (pos obj))
//!               ("type" (get-conj-description (conj-type obj)))))
//!         (neg (conj-neg obj))
//!         (fml (conj-fml obj)))
//!     (unless (or (not neg) (eql neg :null))
//!       (jsown:extend-js js ("neg" neg)))
//!     (unless (or (not fml) (eql fml :null))
//!       (jsown:extend-js js ("fml" fml)))
//!     js))
//! ```
//!
//! Returns a [`serde_json::Value`] object, mirroring the jsown JS object
//! (insertion order preserved via the crate's `preserve_order` feature).
//! `neg`/`fml` map the upstream tri-state `nil`/`t`/`:null` onto
//! `Option<bool>` (`Some(false)`/`Some(true)`/`None`) and are included only
//! when `Some(true)`.

use serde_json::{Map, Value};

use super::conj_prop_dao::ConjProp;
use super::load::get_conj_description;

pub fn conj_prop_json(obj: &ConjProp) -> Value {
    let mut js = Map::new();
    js.insert("pos".to_owned(), Value::String(obj.pos.clone()));
    // dict.lisp:288 — jsown serializes a nil description as []
    js.insert(
        "type".to_owned(),
        match get_conj_description(obj.conj_type) {
            Some(desc) => Value::String(desc.to_owned()),
            None => Value::Array(Vec::new()),
        },
    );
    let neg = obj.neg;
    let fml = obj.fml;
    // dict.lisp:291,293 — (unless (or (not x) (eql x :null)) ...): only the t state extends
    if neg == Some(true) {
        js.insert("neg".to_owned(), Value::Bool(true));
    }
    if fml == Some(true) {
        js.insert("fml".to_owned(), Value::Bool(true));
    }
    Value::Object(js)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(conj_type: i32, pos: &str, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
        ConjProp { id: 0, conj_id: 0, conj_type, pos: pos.to_owned(), neg, fml }
    }

    /// REPL fixtures (.103, `jsown:to-json` of `conj-prop-json` on real
    /// conj_prop rows), 2026-05-24. Rows cover every neg/fml state
    /// (`:null`→None, `t`→Some(true), `nil`→Some(false)); the no-description
    /// row pins jsown's nil→[] rendering of "type".
    #[test]
    fn conj_prop_json_fixtures() {
        let cases = [
            // id=52: neg :null, fml :null
            (prop(13, "v5k", None, None), r#"{"pos":"v5k","type":"Continuative (~i)"}"#),
            // id=3: neg t, fml t
            (prop(1, "v5k", Some(true), Some(true)), r#"{"pos":"v5k","type":"Non-past","neg":true,"fml":true}"#),
            // id=2: neg t, fml nil
            (prop(1, "v5k", Some(true), Some(false)), r#"{"pos":"v5k","type":"Non-past","neg":true}"#),
            // id=1: neg nil, fml t
            (prop(1, "v5k", Some(false), Some(true)), r#"{"pos":"v5k","type":"Non-past","fml":true}"#),
            // id=4: neg nil, fml nil
            (prop(2, "v5k", Some(false), Some(false)), r#"{"pos":"v5k","type":"Past (~ta)"}"#),
            // id=226: neg :null, fml t
            (prop(9, "adj-i", None, Some(true)), r#"{"pos":"adj-i","type":"Volitional","fml":true}"#),
            // id=53: neg t, fml :null
            (prop(52, "v5k", Some(true), None), r#"{"pos":"v5k","type":"Negative Stem","neg":true}"#),
            // no description for conj_type → jsown nil renders as []
            (prop(999, "v5k", None, None), r#"{"pos":"v5k","type":[]}"#),
        ];
        for (obj, expected) in &cases {
            let actual = serde_json::to_string(&conj_prop_json(obj)).unwrap();
            assert_eq!(actual.as_str(), *expected, "conj_type={} pos={}", obj.conj_type, obj.pos);
        }
    }
}
