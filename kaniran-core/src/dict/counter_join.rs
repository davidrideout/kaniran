//! Port of `ichiran/dict:counter-join` (`dict-counters.lisp:3-7,
//! 101-201`).
//!
//! Constructs the kana surface form of a counter expression by
//! splicing `number_kana` and `counter_kana` and applying euphonic
//! transformations (gemination, rendaku, handakuten) keyed by the
//! decimal "digit" (`get-digit n`) and the kana class of
//! `counter_kana`'s first glyph. Three alternative paths fire on a
//! per-counter basis:
//!
//! 1. **Per-digit override** — if the counter has a `digit_opts` entry
//!    matching the digit, or any `:off` entry, the override list is
//!    walked verbatim (geminate / rendaku / handakuten / replace
//!    number-stem / replace counter-kana once `:c` was seen) and the
//!    standard rules are skipped.
//! 2. **Foreign counter** — pure-katakana counters
//!    (`counter.foreign == true`) only geminate before unvoiced
//!    syllables, narrowed per digit (6 / 8 / 10: ka/sa/ta + p-row;
//!    100: ka-row only).
//! 3. **Standard rules** — the long `case digit` block at
//!    `dict-counters.lisp:148-200` covering all digits 1-10000.
//!
//! The Lisp generic dispatches `(counter-text T T T)` to the body
//! above and `(T T T T)` to a plain `concatenate` fallback. Every
//! caller passes a `counter-text` (or subclass), so the (T T T T)
//! method is unreachable in practice; its concatenation behavior is
//! the implicit `format!("{}{}", ...)` returns at the end of every
//! branch in the Rust port.
//!
//! ## Mutation contract
//!
//! [`geminate`] and [`rendaku`] mutate their string in place, mirroring
//! the upstream `(geminate string)` / `(rendaku string)` calls (both
//! default to `:fresh nil`). The Lisp method's `(call-next-method)`
//! invocations rely on this — the `(T T T T)` fallback's
//! `(concatenate 'string number-kana counter-kana)` reads whatever the
//! body mutated in place. The Rust port owns `number_kana` /
//! `counter_kana` as `String` and concatenates them at each return,
//! producing the same result.

use crate::characters::_star_char_class_hash_star_::char_class_hash;
use crate::characters::geminate::geminate;
use crate::characters::kani_kana_class::KanaClass;
use crate::characters::rendaku::{rendaku, Voicing};
use crate::dict::counter_text_class::{Counter, DigitOp, DigitOptKey};
use crate::dict::get_digit::get_digit;
use crate::numbers::_star_digit_to_kana_star_::DIGIT_TO_KANA;
use crate::numbers::_star_power_to_kana_star_::POWER_TO_KANA;

pub fn counter_join(
    counter: &Counter,
    n: i64,
    mut number_kana: String,
    mut counter_kana: String,
) -> String {
    let base = counter.base();
    let digit = get_digit(n);
    // dict-counters.lisp:103 — (gethash (char counter-kana 0) *char-class-hash*)
    let head = counter_kana
        .chars()
        .next()
        .and_then(|c| char_class_hash().get(&c).copied());

    // dict-counters.lisp:104-105 — (assoc digit (digit-opts obj)) /
    // (assoc :off (digit-opts obj))
    let digit_entry = digit.and_then(|d| {
        base.digit_opts
            .iter()
            .find(|e| matches!(e.key, DigitOptKey::Digit(dd) if i64::from(dd) == d))
    });
    let off_present = base
        .digit_opts
        .iter()
        .any(|e| matches!(e.key, DigitOptKey::Off));

    // dict-counters.lisp:106-123 — (when (or off digit-opts) ... loop ...
    // (return-from counter-join (call-next-method obj n number-kana counter-kana)))
    if off_present || digit_entry.is_some() {
        if let Some(entry) = digit_entry {
            let d = digit.expect("digit must be Some when entry matched on Digit(d)");
            let mut mod_counter = false;
            for opt in &entry.ops {
                match opt {
                    DigitOp::Replace(s) => {
                        if mod_counter {
                            counter_kana = s.clone();
                        } else {
                            // dict-counters.lisp:112-116 — splice the
                            // digit/power's own kana stem off the tail
                            // of number-kana and append the override.
                            let stem_chars = digit_kana_char_len(d);
                            let nk_chars: Vec<char> = number_kana.chars().collect();
                            let keep = nk_chars.len().saturating_sub(stem_chars);
                            let mut new_nk: String = nk_chars[..keep].iter().collect();
                            new_nk.push_str(s);
                            number_kana = new_nk;
                        }
                    }
                    DigitOp::Geminate => geminate(&mut number_kana),
                    DigitOp::Rendaku => rendaku(&mut counter_kana, Voicing::Dakuten),
                    DigitOp::Handakuten => rendaku(&mut counter_kana, Voicing::Handakuten),
                    DigitOp::Counter => mod_counter = true,
                }
            }
        }
        // dict-counters.lisp:5-7 — (T T T T) default method: just concat.
        return format!("{}{}", number_kana, counter_kana);
    }

    // dict-counters.lisp:125-146 — (when (counter-foreign obj) ...
    // (return-from counter-join (call-next-method)))
    if base.foreign {
        if let (Some(d), Some(h)) = (digit, head) {
            match d {
                6 if matches!(
                    h,
                    KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                        | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po
                ) => geminate(&mut number_kana),
                8 | 10 if matches!(
                    h,
                    KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                        | KanaClass::Sa | KanaClass::Shi | KanaClass::Su | KanaClass::Se | KanaClass::So
                        | KanaClass::Ta | KanaClass::Chi | KanaClass::Tsu | KanaClass::Te | KanaClass::To
                        | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po
                ) => geminate(&mut number_kana),
                100 if matches!(
                    h,
                    KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                ) => geminate(&mut number_kana),
                _ => {}
            }
        }
        return format!("{}{}", number_kana, counter_kana);
    }

    // dict-counters.lisp:148-200 — standard (case digit ...) over all
    // counter classes that don't have explicit digit-opts.
    if let (Some(d), Some(h)) = (digit, head) {
        match d {
            1 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                | KanaClass::Sa | KanaClass::Shi | KanaClass::Su | KanaClass::Se | KanaClass::So
                | KanaClass::Ta | KanaClass::Chi | KanaClass::Tsu | KanaClass::Te | KanaClass::To => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            3 => {
                if matches!(
                    h,
                    KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho
                ) {
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
            }
            // dict-counters.lisp:160-162 — digit 4 case is `#-(and)`
            // commented out upstream; intentionally a no-op.
            4 => {}
            6 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            8 | 10 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                | KanaClass::Sa | KanaClass::Shi | KanaClass::Su | KanaClass::Se | KanaClass::So
                | KanaClass::Ta | KanaClass::Chi | KanaClass::Tsu | KanaClass::Te | KanaClass::To
                | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            100 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            1000 | 10000 => {
                if matches!(
                    h,
                    KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho
                ) {
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
            }
            _ => {}
        }
    }
    format!("{}{}", number_kana, counter_kana)
}

/// Length, in characters, of the kana stem that represents `digit`
/// inside `number_kana`. Mirrors `dict-counters.lisp:112-114`:
///
/// ```text
/// (length (if (< digit 10)
///             (getf *digit-to-kana* digit)
///             (getf *power-to-kana* (round (log digit 10)))))
/// ```
///
/// Lisp `length` on a `simple-string` is character count (= code-point
/// count under SBCL), so use [`str::chars`] / [`Iterator::count`] —
/// not `String::len`, which is byte count and would split multi-byte
/// kana wrong (every entry is in the BMP, 3 bytes per char in UTF-8).
fn digit_kana_char_len(digit: i64) -> usize {
    if digit < 10 {
        DIGIT_TO_KANA[digit as usize].chars().count()
    } else {
        let exp = (digit as f64).log10().round() as u8;
        POWER_TO_KANA
            .iter()
            .find(|(e, _)| *e == exp)
            .map(|(_, s)| s.chars().count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    //! Tests cover the per-branch logic that fixture replay alone
    //! wouldn't pin clearly: the `:c` mod-counter sequencing, the
    //! `:off` early-out, the digit-stem replacement char-length math.
    //! Bulk behavioral coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/counter_join.parquet`
    //! (131k rows) — replayed by `audit_fixtures`.
    use super::*;
    use crate::dict::counter_text_class::{
        Counter, CounterText, DigitOp, DigitOptEntry, DigitOptKey, Common,
    };

    fn make_counter(digit_opts: Vec<DigitOptEntry>, foreign: bool) -> Counter {
        Counter::Base(CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: "0".into(),
            number: 0,
            source: None,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts,
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign,
        })
    }

    #[test]
    fn off_skips_all_euphony() {
        // dict-counters.lisp:456 — counter-text "立て" :digit-opts ((:off)).
        // For digit 1 + ka-row, the standard rule would geminate; :off
        // suppresses it.
        let c = make_counter(
            vec![DigitOptEntry { key: DigitOptKey::Off, ops: Vec::new() }],
            false,
        );
        let out = counter_join(&c, 1, "いち".into(), "か".into());
        assert_eq!(out, "いちか");
    }

    #[test]
    fn digit_replace_swaps_number_stem() {
        // dict-counters.lisp:386 — counter-text "時" digit-opts ((4 "よ")).
        // For digit 4: number-kana "よん" → strip stem "よん" (len 2),
        // append "よ" → "よ".
        let c = make_counter(
            vec![DigitOptEntry {
                key: DigitOptKey::Digit(4),
                ops: vec![DigitOp::Replace("よ".into())],
            }],
            false,
        );
        let out = counter_join(&c, 4, "よん".into(), "じ".into());
        assert_eq!(out, "よじ");
    }

    #[test]
    fn mod_counter_then_string_replaces_counter_kana() {
        // dict-counters.lisp:425 — counter-text "羽" digit-opts
        // ((3 :c "ば") ...). For digit 3: :c flips mod-counter, then
        // the string replaces counter-kana itself (not number-kana).
        let c = make_counter(
            vec![DigitOptEntry {
                key: DigitOptKey::Digit(3),
                ops: vec![DigitOp::Counter, DigitOp::Replace("ば".into())],
            }],
            false,
        );
        let out = counter_join(&c, 3, "さん".into(), "わ".into());
        assert_eq!(out, "さんば");
    }

    #[test]
    fn power_digit_replace_strips_power_stem() {
        // dict-counters.lisp:471 — counter-text "世紀" digit-opts
        // ((10 "じっ")). For digit 10: number-kana ends in "じゅう"
        // (len 3 chars); strip and append "じっ".
        let c = make_counter(
            vec![DigitOptEntry {
                key: DigitOptKey::Digit(10),
                ops: vec![DigitOp::Replace("じっ".into())],
            }],
            false,
        );
        let out = counter_join(&c, 10, "じゅう".into(), "せいき".into());
        assert_eq!(out, "じっせいき");
    }

    #[test]
    fn foreign_geminates_only_for_specific_heads() {
        // dict-counters.lisp:127-130 — foreign + digit 6 + ka-row: geminate.
        let c = make_counter(Vec::new(), true);
        let out = counter_join(&c, 6, "ろく".into(), "か".into());
        assert_eq!(out, "ろっか");
        // Same setup, t-row head: foreign-digit-6 doesn't fire (only ka/p-row).
        let out2 = counter_join(&c, 6, "ろく".into(), "た".into());
        assert_eq!(out2, "ろくた");
    }

    #[test]
    fn standard_digit_1_geminates_and_handakutens_h_row() {
        // dict-counters.lisp:148-156 — digit 1 + ha-row: geminate
        // number-kana, handakuten counter-kana.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 1, "いち".into(), "ほん".into());
        assert_eq!(out, "いっぽん");
    }

    #[test]
    fn standard_digit_4_is_noop() {
        // dict-counters.lisp:160-162 — digit 4 case is commented out
        // upstream (`#-(and)` reader-conditional). Even h-row stays put.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 4, "よん".into(), "ほん".into());
        assert_eq!(out, "よんほん");
    }

    #[test]
    fn n_zero_no_digit_match_just_concatenates() {
        // get-digit 0 returns nil (n divisible by 10^8); the body's case
        // arms all skip → plain concat of れい + counter-kana.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 0, "れい".into(), "ほん".into());
        assert_eq!(out, "れいほん");
    }

    #[test]
    fn empty_counter_kana_no_head_lookup() {
        // (char counter-kana 0) on "" would error in Lisp — guarded
        // upstream by populator never producing empty counter-kana, but
        // pin the Rust head=None path here so it's not a future panic.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 1, "いち".into(), String::new());
        assert_eq!(out, "いち");
    }
}
