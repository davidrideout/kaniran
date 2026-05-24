//! Port of `ichiran/dict:conj-info-short` (`dict.lisp:277`).
//!
//! Formats a [`ConjProp`] as `[<pos>] <description>` followed by an
//! optional ` Affirmative`/` Negative` and ` Plain`/` Formal` selected
//! by the nullable `neg`/`fml` flags (`db-null` → omitted).

use super::conj_prop_dao::ConjProp;
use super::get_conj_description::get_conj_description;

pub fn conj_info_short(obj: &ConjProp) -> String {
    // dict.lisp:277 — "[~a] ~a~@[~[ Affirmative~; Negative~]~]~@[~[ Plain~; Formal~]~]"
    let neg = match obj.neg {
        Some(false) => " Affirmative",
        Some(true) => " Negative",
        None => "",
    };
    let fml = match obj.fml {
        Some(false) => " Plain",
        Some(true) => " Formal",
        None => "",
    };
    format!(
        "[{}] {}{}{}",
        obj.pos,
        // ~a of nil prints "NIL"
        get_conj_description(obj.conj_type).unwrap_or("NIL"),
        neg,
        fml,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(pos: &str, conj_type: i32, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            conj_type,
            pos: pos.to_string(),
            neg,
            fml,
        }
    }

    /// REPL fixtures (.103, ichiran/dict::conj-info-short on conj_prop
    /// rows), 2026-05-24. Covers every neg/fml state — Some(false)
    /// (Lisp nil), Some(true) (Lisp t), None (db-null) — plus the
    /// missing-description path (`~a` of nil → "NIL").
    #[test]
    fn conj_info_short_fixtures() {
        let cases: &[(ConjProp, &str)] = &[
            (
                prop("v5k", 2, Some(false), Some(false)),
                "[v5k] Past (~ta) Affirmative Plain",
            ),
            (
                prop("v5k", 1, Some(false), Some(true)),
                "[v5k] Non-past Affirmative Formal",
            ),
            (
                prop("v5k", 1, Some(true), Some(false)),
                "[v5k] Non-past Negative Plain",
            ),
            (
                prop("v5k", 1, Some(true), Some(true)),
                "[v5k] Non-past Negative Formal",
            ),
            (
                prop("adj-i", 3, Some(false), None),
                "[adj-i] Conjunctive (~te) Affirmative",
            ),
            (
                prop("v5k", 52, Some(true), None),
                "[v5k] Negative Stem Negative",
            ),
            (
                prop("adj-i", 9, None, Some(false)),
                "[adj-i] Volitional Plain",
            ),
            (
                prop("adj-i", 9, None, Some(true)),
                "[adj-i] Volitional Formal",
            ),
            (prop("v5k", 13, None, None), "[v5k] Continuative (~i)"),
            (
                prop("v5k", 999, Some(false), Some(false)),
                "[v5k] NIL Affirmative Plain",
            ),
        ];
        for (obj, expected) in cases {
            assert_eq!(&conj_info_short(obj), expected, "obj={obj:?}");
        }
    }
}
