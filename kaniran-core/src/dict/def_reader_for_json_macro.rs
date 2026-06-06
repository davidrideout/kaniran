//! Port of `ichiran/dict:def-reader-for-json` (`dict.lisp:1292`).
//!
//! Reads the value at `slot` from a `word-info-json` object, panicking
//! on an absent key like `jsown:val`'s error.

use serde_json::Value;

pub fn def_reader_for_json<'a>(obj: &'a Value, slot: &str) -> &'a Value {
    obj.get(slot)
        .unwrap_or_else(|| panic!("jsown:val: key {slot:?} not present in object"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // REPL fixtures (.103, `(jsown:val (word-info-json (word-info-from-text W)) slot)`
    // after `(init-suffixes t t)`), 2026-05-25. jsown renders CL nil as `[]`, so the
    // serde_json object `word-info-json` builds matches the captured shapes; each
    // generated reader returns the value pinned here.

    #[test]
    fn reads_each_slot() {
        // 薔薇 — single-seq noun: scalar slots, every nil slot serialized as [].
        let bara = json!({
            "type":"KANJI","text":"薔薇","truetext":"薔薇","kana":"ばら",
            "seq":1571760,"conjugations":[],"score":143,"components":[],
            "alternative":[],"primary":true,"start":0,"end":2,"counter":[],"skipped":0
        });
        let cases: &[(&str, Value)] = &[
            ("text", json!("薔薇")),
            ("truetext", json!("薔薇")),
            ("kana", json!("ばら")),
            ("seq", json!(1571760)),
            ("score", json!(143)),
            ("components", json!([])),
            ("alternative", json!([])),
            ("primary", json!(true)),
            ("start", json!(0)),
            ("end", json!(2)),
            ("counter", json!([])),
            ("skipped", json!(0)),
        ];
        for &(slot, ref expected) in cases {
            assert_eq!(def_reader_for_json(&bara, slot), expected, "slot={slot}");
        }
    }

    #[test]
    fn reads_multi_value_slots() {
        // 一人 — alternative reading: kana/seq are arrays, components is the
        // two-child array (counter on the second child), alternative t,
        // truetext nil→[].
        let hitori = json!({
            "type":"KANJI","text":"一人","truetext":[],"kana":["ひとり"],
            "seq":[1576150,2149890],"conjugations":[],"score":312,
            "components":[
                {"type":"KANJI","text":"一人","truetext":"一人","kana":"ひとり","seq":1576150,
                 "conjugations":[],"score":312,"components":[],"alternative":[],"primary":true,
                 "start":0,"end":2,"counter":[],"skipped":0},
                {"type":"KANJI","text":"一人","truetext":[],"kana":"ひとり","seq":2149890,
                 "conjugations":[],"score":208,"components":[],"alternative":[],"primary":true,
                 "start":0,"end":2,"counter":["Value: 1",[]],"skipped":0}
            ],
            "alternative":true,"primary":true,"start":0,"end":2,"counter":[],"skipped":0
        });
        assert_eq!(def_reader_for_json(&hitori, "kana"), &json!(["ひとり"]));
        assert_eq!(def_reader_for_json(&hitori, "seq"), &json!([1576150, 2149890]));
        assert_eq!(def_reader_for_json(&hitori, "alternative"), &json!(true));
        assert_eq!(def_reader_for_json(&hitori, "truetext"), &json!([]));
        let components = def_reader_for_json(&hitori, "components");
        let second_child = &components.as_array().expect("components is an array")[1];
        assert_eq!(
            def_reader_for_json(second_child, "counter"),
            &json!(["Value: 1", []])
        );
    }

    #[test]
    #[should_panic(expected = "not present")]
    fn panics_on_missing_key() {
        // jsown:val errors on an absent key.
        let obj = json!({"text": "x"});
        def_reader_for_json(&obj, "nonexistent");
    }
}
