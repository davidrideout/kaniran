use super::*;
use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
use crate::dict::counters::kani_counter_args::CounterArgs;

// --- counter_text_class ---
// These tests exercise `Counter::new` on its own: given a counter recipe
// (`CounterArgs`) and a number, it builds the right `Counter` variant with the
// expected fields filled in.

#[test]
fn base_text_arm() {
    // A plain text counter copies its recipe fields (text, kana) straight onto
    // the built counter.
    let args = CounterArgs::new(CounterClass::Text, "個", "か");
    let c = Counter::new(&args, "5").unwrap();
    let base = c.base();
    assert!(matches!(c, Counter::Base(_)));
    assert_eq!(base.text, "個");
    assert_eq!(base.kana, "か");
    assert_eq!(base.number_text, "5");
    assert_eq!(base.number, 5);
    assert!(!base.ordinalp);
    assert_eq!(base.suffix, None);
    assert!(base.allowed.is_empty());
    assert!(base.digit_opts.is_empty());
}

#[test]
fn number_text_arm_empty_seed() {
    // A number-text counter with empty text and kana still parses its number.
    let args = CounterArgs::new(CounterClass::NumberText, "", "");
    let c = Counter::new(&args, "42").unwrap();
    assert!(matches!(c, Counter::NumberText(_)));
    assert_eq!(c.base().number, 42);
    assert_eq!(c.base().text, "");
    assert_eq!(c.base().kana, "");
}

#[test]
fn days_kun_initform_fills_when_recipe_omits_allowed() {
    let args = CounterArgs::new(CounterClass::DaysKun, "日", "か");
    let c = Counter::new(&args, "1").unwrap();
    assert!(matches!(c, Counter::DaysKun(_)));
    assert_eq!(
        c.base().allowed,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 14, 20, 24, 30]
    );
    assert_eq!(c.base().number, 1);
}

#[test]
fn days_kun_explicit_allowed_overrides_initform() {
    // A recipe that sets `allowed` wins over the class default.
    let args = CounterArgs::new(CounterClass::DaysKun, "日", "か").allowed(vec![99]);
    let c = Counter::new(&args, "1").unwrap();
    assert_eq!(c.base().allowed, vec![99]);
}

#[test]
fn months_initforms_fill_when_recipe_omits() {
    let args = CounterArgs::new(CounterClass::Months, "月", "がつ");
    let c = Counter::new(&args, "4").unwrap();
    assert!(matches!(c, Counter::Months(_)));
    let base = c.base();
    assert_eq!(base.allowed, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    assert_eq!(base.digit_opts.len(), 3);
    assert_eq!(base.digit_opts[0].key, DigitOptKey::Digit(4));
    assert_eq!(
        base.digit_opts[0].ops,
        vec![DigitOp::Replace("し".to_string())]
    );
    assert_eq!(base.digit_opts[1].key, DigitOptKey::Digit(7));
    assert_eq!(
        base.digit_opts[1].ops,
        vec![DigitOp::Replace("しち".to_string())]
    );
    assert_eq!(base.digit_opts[2].key, DigitOptKey::Digit(9));
    assert_eq!(
        base.digit_opts[2].ops,
        vec![DigitOp::Replace("く".to_string())]
    );
    assert_eq!(base.number, 4);
}

#[test]
fn hifumi_propagates_digit_set_from_recipe() {
    let args = CounterArgs::new(CounterClass::Hifumi, "株", "かぶ").digit_set(vec![1, 2]);
    let c = Counter::new(&args, "1").unwrap();
    match c {
        Counter::Hifumi(h) => {
            assert_eq!(h.digit_set, vec![1, 2]);
            assert_eq!(h.base.number, 1);
        }
        other => panic!("expected Hifumi, got {:?}", other),
    }
}

#[test]
#[should_panic(expected = "counter-hifumi requires non-empty :digit-set")]
fn hifumi_panics_on_empty_digit_set() {
    // A hifumi counter has no default digit-set, so building one without it
    // panics.
    let args = CounterArgs::new(CounterClass::Hifumi, "株", "かぶ");
    let _ = Counter::new(&args, "1");
}

#[test]
fn parse_number_failure_returns_err() {
    // A non-numeric number-text fails to build the counter.
    let args = CounterArgs::new(CounterClass::Text, "個", "か");
    let err = Counter::new(&args, "X").unwrap_err();
    assert_eq!(err.text, "X");
}

#[test]
fn parse_number_handles_value_above_i32() {
    // A number larger than the i32 range flows through intact.
    let args = CounterArgs::new(CounterClass::Text, "個", "か");
    let c = Counter::new(&args, "12423000430").unwrap();
    assert_eq!(c.base().number, 12_423_000_430);
}

#[test]
fn ordinal_recipe_propagates_to_base() {
    // An ordinal recipe sets ordinalp, suffix, and digit-opts on the counter.
    let args = CounterArgs::new(CounterClass::Text, "階目", "かい")
        .ordinalp(true)
        .suffix("め")
        .digit_opts(vec![DigitOptEntry {
            key: DigitOptKey::Digit(3),
            ops: vec![DigitOp::Rendaku],
        }]);
    let c = Counter::new(&args, "2").unwrap();
    let base = c.base();
    assert!(base.ordinalp);
    assert_eq!(base.suffix.as_deref(), Some("め"));
    assert_eq!(base.digit_opts.len(), 1);
    assert_eq!(base.digit_opts[0].key, DigitOptKey::Digit(3));
    assert_eq!(base.digit_opts[0].ops, vec![DigitOp::Rendaku]);
    assert_eq!(base.number, 2);
}

// --- counter_hifumi_class ---
fn make_ct(number: u64, kana: &str) -> CounterText {
    CounterText {
        text: kana.to_string(),
        kana: kana.to_string(),
        number_text: number.to_string(),
        number,
        source: None as Option<CounterSource>,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed: Vec::new(),
        foreign: false,
    }
}

/// When the value is in the digit-set and within 1..=10, the reading is the
/// native-number prefix followed by the counter's kana.
#[test]
fn in_digit_set_in_range_uses_prefix_plus_kana() {
    let c = Counter::Hifumi(CounterHifumi {
        base: make_ct(1, "かぶ"),
        digit_set: vec![1, 2],
    });
    assert_eq!(c.get_kana(), "ひとかぶ");
}

/// When the value is in the digit-set but outside 1..=10, there is no prefix,
/// so the reading is the counter's kana alone (it does not fall through to the
/// base reading).
#[test]
fn in_digit_set_outside_range_is_kana_only_not_call_next_method() {
    let c = Counter::Hifumi(CounterHifumi {
        base: make_ct(11, "かぶ"),
        digit_set: vec![11],
    });
    assert_eq!(c.get_kana(), "かぶ");
}

/// When the value is outside the digit-set, the reading falls through to the
/// base counter behavior (number kana joined with counter kana).
#[test]
fn outside_digit_set_falls_through_to_counter_join() {
    let c = Counter::Hifumi(CounterHifumi {
        base: make_ct(5, "かぶ"),
        digit_set: vec![1, 2],
    });
    let result = c.get_kana();
    assert_ne!(result, "かぶ");
    assert_ne!(result, "いつかぶ");
    assert!(
        result.contains("かぶ"),
        "expected counter-kana, got {:?}",
        result
    );
}
