//! Port of `ichiran/dict:sense-exists-p` (`dict-load.lisp:82`).
//!
//! Returns true when some sense in `senses` has the same parts of
//! speech and glosses as the candidate. A sense with no parts of
//! speech of its own inherits them from the most recent earlier sense
//! that had them.

use crate::characters::join::join;
use crate::dict::get_senses_raw::RawSense;

pub fn sense_exists_p(
    senses: &[RawSense],
    positions: &[String],
    glosses: &[String],
) -> bool {
    let glosses_str = join("; ", glosses);
    let mut rpos: Option<&[String]> = None;
    let mut first = true;
    for sense in senses {
        let pos: Option<&[String]> = sense
            .props
            .iter()
            .find(|(tag, _)| tag == "pos")
            .map(|(_, vals)| vals.as_slice());
        // dict-load.lisp:88 (for rpos = pos then (or pos rpos))
        rpos = if first { pos } else { pos.or(rpos) };
        first = false;
        let pos_match = match rpos {
            Some(rp) => rp == positions,
            None => positions.is_empty(),
        };
        if pos_match && glosses_str == sense.gloss {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(ord: i32, gloss: &str, props: &[(&str, &[&str])]) -> RawSense {
        RawSense {
            ord,
            gloss: gloss.to_string(),
            props: props
                .iter()
                .map(|(tag, vals)| {
                    (
                        (*tag).to_string(),
                        vals.iter().map(|s| (*s).to_string()).collect(),
                    )
                })
                .collect(),
        }
    }

    fn sv(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| (*s).to_string()).collect()
    }

    // REPL fixtures (.103, ichiran/dict::sense-exists-p), 2026-05-31.
    #[test]
    fn single_sense_exact_match() {
        // (get-senses-raw 1582710) =>
        //   ((:ORD 0 :GLOSS "Japan" :PROPS (("pos" "n"))))
        let senses = vec![raw(0, "Japan", &[("pos", &["n"])])];
        assert!(sense_exists_p(&senses, &sv(&["n"]), &sv(&["Japan"])));
    }

    #[test]
    fn single_sense_mismatches_and_empty_inputs() {
        // 1582710 baseline with negative cases.
        let senses = vec![raw(0, "Japan", &[("pos", &["n"])])];
        let cases: &[(&str, &[&str], &[&str], bool)] = &[
            ("diff-gloss", &["n"], &["Korea"], false),
            ("diff-pos", &["vt"], &["Japan"], false),
            ("empty-glosses", &["n"], &[], false),
            ("empty-positions", &[], &["Japan"], false),
        ];
        for (label, pos, gl, expected) in cases {
            assert_eq!(
                sense_exists_p(&senses, &sv(pos), &sv(gl)),
                *expected,
                "{label}",
            );
        }
    }

    #[test]
    fn multi_pos_value_lists_must_match_in_order() {
        // (get-senses-raw 1000300) — two senses with `pos` value-lists
        // in opposite orders.
        let senses = vec![
            raw(
                0,
                "to treat; to handle; to deal with",
                &[("stagk", &["遇う"]), ("pos", &["v5u", "vt"])],
            ),
            raw(
                1,
                "to arrange; to decorate (with); to adorn (with); to dress (with); to garnish (with)",
                &[("pos", &["vt", "v5u"])],
            ),
        ];
        assert!(sense_exists_p(
            &senses,
            &sv(&["v5u", "vt"]),
            &sv(&["to treat", "to handle", "to deal with"]),
        ));
        assert!(sense_exists_p(
            &senses,
            &sv(&["vt", "v5u"]),
            &sv(&[
                "to arrange",
                "to decorate (with)",
                "to adorn (with)",
                "to dress (with)",
                "to garnish (with)",
            ]),
        ));
        // first sense's joined gloss with sense-2's pos order — no match
        assert!(!sense_exists_p(
            &senses,
            &sv(&["vt", "v5u"]),
            &sv(&["to treat", "to handle", "to deal with"]),
        ));
    }

    #[test]
    fn second_sense_with_no_props_inherits_prior_pos() {
        // (get-senses-raw 1447690) =>
        //   ((:ORD 0 :GLOSS "Tokyo" :PROPS (("pos" "n")))
        //    (:ORD 1 :GLOSS "Tokyo Metropolis" :PROPS NIL))
        let senses = vec![
            raw(0, "Tokyo", &[("pos", &["n"])]),
            raw(1, "Tokyo Metropolis", &[]),
        ];
        let cases: &[(&str, &[&str], &[&str], bool)] = &[
            ("sense1-match", &["n"], &["Tokyo"], true),
            ("sense2-rpos-fallback", &["n"], &["Tokyo Metropolis"], true),
            ("sense2-wrong-pos", &["v5u"], &["Tokyo Metropolis"], false),
            ("sense1-wrong-pos", &["vt"], &["Tokyo"], false),
        ];
        for (label, pos, gl, expected) in cases {
            assert_eq!(
                sense_exists_p(&senses, &sv(pos), &sv(gl)),
                *expected,
                "{label}",
            );
        }
    }

    #[test]
    fn first_sense_without_pos_does_not_match_pos_request() {
        // Synthetic: first sense has no pos; rpos starts as nil, so a
        // non-empty positions request can't match sense 1 even if its
        // gloss matches.
        let senses = vec![
            raw(0, "a", &[]),
            raw(1, "b", &[("pos", &["n"])]),
        ];
        assert!(sense_exists_p(&senses, &sv(&["n"]), &sv(&["b"])));
        assert!(!sense_exists_p(&senses, &sv(&["n"]), &sv(&["a"])));
    }

    #[test]
    fn glosses_joined_with_separator_before_compare() {
        // Synthetic input plist, output pinned by REPL probe on .103
        // (2026-05-31). Pins the `(join "; " glosses)` step: the input
        // glosses list is joined with the same "; " separator the
        // upstream string_agg produced into sense.gloss.
        let senses = vec![raw(
            0,
            "Japan; Land of the Rising Sun",
            &[("pos", &["n"])],
        )];
        assert!(sense_exists_p(
            &senses,
            &sv(&["n"]),
            &sv(&["Japan", "Land of the Rising Sun"]),
        ));
        // A single-element input whose value already contains the
        // separator joins to itself (no separator inserted), so it
        // also matches — same REPL probe pinned this.
        assert!(sense_exists_p(
            &senses,
            &sv(&["n"]),
            &sv(&["Japan; Land of the Rising Sun"]),
        ));
    }

    #[test]
    fn empty_inputs_match_when_sense_is_also_empty() {
        // Synthetic: sense with empty gloss and no props matches the
        // (empty, empty) request because (equal nil nil) is true.
        let senses = vec![raw(0, "", &[])];
        assert!(sense_exists_p(&senses, &[], &[]));
    }

    #[test]
    fn empty_sense_list_never_matches() {
        assert!(!sense_exists_p(&[], &sv(&["n"]), &sv(&["foo"])));
    }
}
