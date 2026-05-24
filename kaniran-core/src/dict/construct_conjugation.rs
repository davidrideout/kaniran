//! Port of `ichiran/dict:construct-conjugation` (`dict-load.lisp:284`).
//!
//! Assemble a conjugated reading from a dictionary `word` and a
//! `ConjugationRule`: drop `stem` trailing characters (one extra when
//! the applicable euphonic fragment is non-empty), then append the
//! euphonic fragment (`euphr` when the last two characters are kana,
//! `euphk` otherwise) and `okuri`.
//!
//! Lisp `length` / `subseq` index by character; the port collects
//! `chars()` so offsets stay character-based for multi-byte readings.

use super::conjugation_rule_struct::ConjugationRule;
use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;

pub fn construct_conjugation(word: &str, rule: &ConjugationRule) -> String {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    // (subseq word (max 0 (- (length word) 2))) — last two characters
    let last_two: String = chars[len.saturating_sub(2)..].iter().collect();
    let iskana = test_word(&last_two, CharClass::Kana);
    let euphr = &rule.euphr;
    let euphk = &rule.euphk;
    let stem = rule.stem
        + if (iskana && !euphr.is_empty()) || (!iskana && !euphk.is_empty()) {
            1
        } else {
            0
        };
    let mut result: String = chars[..len - stem as usize].iter().collect();
    result.push_str(if iskana { euphr } else { euphk });
    result.push_str(&rule.okuri);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(stem: i32, okuri: &str, euphr: &str, euphk: &str) -> ConjugationRule {
        ConjugationRule {
            pos: 0,
            conj: 0,
            neg: false,
            fml: false,
            onum: 1,
            stem,
            okuri: okuri.to_string(),
            euphr: euphr.to_string(),
            euphk: euphk.to_string(),
        }
    }

    /// REPL (.103, `(construct-conjugation reading rule)` over real
    /// conjugation rows): exercises every branch — kana vs kanji ending
    /// (`iskana`), euphr vs euphk selection, the +1 stem bump when the
    /// applicable euphonic fragment is non-empty (and no bump when it is
    /// empty), the plain no-euphonic case, and the `(max 0 …)` guard on
    /// a one-character reading.
    #[test]
    fn construct_conjugation_paths() {
        // (reading, rule, expected)
        let cases: &[(&str, ConjugationRule, &str)] = &[
            // vs-i conj=5: euphr non-empty, kana ending -> euphr + bump
            ("する", rule(1, "る", "でき", "出来"), "できる"),
            // vs-i conj=5: euphk non-empty, kanji ending -> euphk + bump
            ("為る", rule(1, "る", "でき", "出来"), "出来る"),
            // adj-ix conj=2: euphr non-empty, kana ending -> euphr + bump
            ("いい", rule(1, "かった", "よ", ""), "よかった"),
            // adj-ix conj=2: euphk empty, kanji ending -> no bump, euphk ""
            ("良い", rule(1, "かった", "よ", ""), "良かった"),
            // vk conj=1: euphr non-empty, kana ending -> euphr + bump
            ("くる", rule(1, "ます", "き", ""), "きます"),
            // vk conj=1: euphk empty, kanji ending -> no bump, euphk ""
            ("来る", rule(1, "ます", "き", ""), "来ます"),
            // v5u conj=3: no euphonic fragments, kana ending -> no bump
            ("かう", rule(1, "って", "", ""), "かって"),
            // v5u conj=3: no euphonic fragments, kanji ending -> no bump
            ("買う", rule(1, "って", "", ""), "買って"),
            // one-character reading: (max 0 (- 1 2)) guard, stem=1 -> prefix ""
            ("る", rule(1, "わ", "", ""), "わ"),
        ];
        for (reading, conj_rule, expected) in cases {
            assert_eq!(
                construct_conjugation(reading, conj_rule),
                *expected,
                "reading={reading}"
            );
        }
    }
}
