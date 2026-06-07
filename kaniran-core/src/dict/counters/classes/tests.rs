use super::*;
use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
use crate::dict::counters::kani_counter_args::CounterArgs;

// --- counter_text_class ---
// Ground truth captured via `./ichiran-repl.sh` heredoc against the
// .103 ichiran install. Each test case mirrors a (number, counter)
// probe call to `find-counter`; the asserted slot values are read
// from the materialized counter-text instance(s) Lisp returned.
//
// Tests target `Counter::new`'s logic in isolation: the constructor
// takes a `CounterArgs` recipe + number-text and produces a
// `Counter` enum variant with the right slots populated. The
// upstream `find-counter` flow (lookup recipe by text key + verify)
// is not under test here — that lands with its own port.

#[test]
fn base_text_arm() {
    // dict-counters.lisp:278 — find-counter "5" "個" returns 2 COUNTER-TEXT
    // recipes (kana=か, kana=こ); slots match per-recipe directly.
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
    // dict-counters.lisp:241 — (add-args "" 'number-text) seeds the cache
    // with an empty key. find-counter "42" "" → NUMBER-TEXT, num=42.
    let args = CounterArgs::new(CounterClass::NumberText, "", "");
    let c = Counter::new(&args, "42").unwrap();
    assert!(matches!(c, Counter::NumberText(_)));
    assert_eq!(c.base().number, 42);
    assert_eq!(c.base().text, "");
    assert_eq!(c.base().kana, "");
}

#[test]
fn days_kun_initform_fills_when_recipe_omits_allowed() {
    // dict-counters.lisp:687 (defclass counter-days-kun
    //   ((allowed :initform '(1 2 3 4 5 6 7 8 9 10 14 20 24 30)))).
    // Lisp probe of (find-counter "1" "日") returns COUNTER-DAYS-KUN with
    // exactly that allowed list materialized.
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
    // CLOS :initform fires only when no :initarg is passed. A recipe that
    // sets :allowed wins.
    let args = CounterArgs::new(CounterClass::DaysKun, "日", "か").allowed(vec![99]);
    let c = Counter::new(&args, "1").unwrap();
    assert_eq!(c.base().allowed, vec![99]);
}

#[test]
fn months_initforms_fill_when_recipe_omits() {
    // dict-counters.lisp:721-723 (defclass counter-months
    //   ((allowed :initform '(1..12))
    //    (digit-opts :initform '((4 "し") (7 "しち") (9 "く"))))).
    // Lisp probe of (find-counter "1" "月") materializes both.
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
    // dict-counters.lisp:541 — (args 'counter-hifumi "株" "かぶ" :digit-set '(1 2)).
    // Lisp probe of (find-counter "1" "株") returns COUNTER-HIFUMI with
    // digit-set=(1 2) populated from the :digit-set initarg.
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
    // dict-counters.lisp:518-519 — :digit-set has no :initform; upstream
    // make-instance without :digit-set leaves the slot unbound.
    let args = CounterArgs::new(CounterClass::Hifumi, "株", "かぶ");
    let _ = Counter::new(&args, "1");
}

#[test]
fn parse_number_failure_returns_err() {
    // dict-counters.lisp:51 — initialize-instance :after counter-text calls
    // parse-number on number-text; invalid input raises not-a-number,
    // which the Rust port surfaces as Err(NotANumber).
    let args = CounterArgs::new(CounterClass::Text, "個", "か");
    let err = Counter::new(&args, "X").unwrap_err();
    assert_eq!(err.text, "X");
}

#[test]
fn parse_number_handles_value_above_i32() {
    // numbers.lisp:74 (parse-number) — returns u64; the roundtrip test in
    // parse_number.rs covers 12_423_000_430. Pin that the value flows
    // through Counter::new into the number slot intact.
    let args = CounterArgs::new(CounterClass::Text, "個", "か");
    let c = Counter::new(&args, "12423000430").unwrap();
    assert_eq!(c.base().number, 12_423_000_430);
}

#[test]
fn ordinal_recipe_propagates_to_base() {
    // dict-counters.lisp:233-269 — *counter-cache* ordinal pass adds
    // <counter>目 derivatives with ordinalp=t and suffix=め (concatenated
    // onto any pre-existing suffix). Lisp probe of (find-counter "2" "階目")
    // returns COUNTER-TEXT with ord=T, suffix="め", digit-opts=((3 R)).
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

/// Hifumi path with `value` covered by the 1..=10 prefix table:
/// `prefix + counter.kana`.
#[test]
fn in_digit_set_in_range_uses_prefix_plus_kana() {
    let c = Counter::Hifumi(CounterHifumi {
        base: make_ct(1, "かぶ"),
        digit_set: vec![1, 2],
    });
    assert_eq!(c.get_kana(), "ひとかぶ");
}

/// Hifumi path with `value` in `digit-set` but OUTSIDE 1..=10:
/// upstream `(case value ...)` returns nil, and
/// `(concatenate 'string nil counter-kana)` yields just
/// `counter-kana` — NOT call-next-method.
#[test]
fn in_digit_set_outside_range_is_kana_only_not_call_next_method() {
    let c = Counter::Hifumi(CounterHifumi {
        base: make_ct(11, "かぶ"),
        digit_set: vec![11],
    });
    assert_eq!(c.get_kana(), "かぶ");
}

/// Hifumi path with `value` OUTSIDE `digit-set`: fall through to
/// `(call-next-method)` — counter-text base primary
/// (counter_join over number-kana + counter-kana).
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
