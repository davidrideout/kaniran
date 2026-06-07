use super::*;
use crate::dict::counters::classes::{
    Common, Counter, CounterText, DigitOp, DigitOptEntry, DigitOptKey,
};

// --- counter_join ---
// Tests cover the per-branch logic that fixture replay alone
// wouldn't pin clearly: the `:c` mod-counter sequencing, the
// `:off` early-out, the digit-stem replacement char-length math.
// Bulk behavioral coverage lives in
// `corpus/extracted_counter_2026_05_08/dict/counter_join.parquet`
// (131k rows) — replayed by `audit_fixtures`.

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
        vec![DigitOptEntry {
            key: DigitOptKey::Off,
            ops: Vec::new(),
        }],
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
