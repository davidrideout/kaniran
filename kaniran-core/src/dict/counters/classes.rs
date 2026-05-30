//! Port of `ichiran/dict:counter-text` (`dict-counters.lisp:9`).
//!
//! Base type for every entry the `*counter-cache*` populator stores
//! and that `find-counter` instantiates per-query. Carries the surface
//! forms (`text`, `kana`, `number_text`) plus the metadata that drives
//! `counter-join`'s euphonic transformations (`digit_opts`, `foreign`,
//! `accepts_suffixes`) and `find-counter`'s filtering (`allowed`,
//! `common`).
//!
//! Construction goes through [`Counter::new`], mirroring
//! `find-counter`'s sole `make-instance` call site
//! (`dict-counters.lisp:278`). It branches on [`CounterArgs::class`]
//! to wrap the shared base into the right variant — populating
//! subclass-specific slots like [`CounterHifumi::digit_set`] in the
//! process — and runs the `initialize-instance :after`
//! (`dict-counters.lisp:51`) that fills the `number` slot from
//! `(parse-number number-text)`. The base-only construction step
//! [`CounterText::from_args`] is kept private.
//!
//! [`CounterArgs::class`]: crate::dict::kani::CounterArgs#structfield.class
//! [`CounterHifumi::digit_set`]: crate::dict::counters::subclasses::CounterHifumi#structfield.digit_set
//!
//! Slot-typing divergences from the Lisp:
//! - `foreign`: every caller (`(when (counter-foreign obj) ...)` in
//!   `counter-join`) treats it as a predicate, so [`bool`], even
//!   though the populator passes the matching `seq` integer or `t`
//!   upstream.
//! - `common`: the slot accepts `nil` (delegate to source), `:null`
//!   (explicitly null), or an integer score. Modeled as a 3-variant
//!   enum [`Common`].
//! - `digit_opts`: closed set of tagged ops (`:g`, `:r`, `:h`, `:c`,
//!   plus literal-string replacements) keyed by digit or `:off`,
//!   modeled as enums [`DigitOp`] / [`DigitOptKey`]. Slot docstring
//!   lists `:d` (dakuten) but no call site or method handles it, so
//!   it is omitted; add it if the data ever uses it.
//! - `source`: holds whichever JMdict reading row produced this
//!   counter (kanji-text or kana-text), or [`None`] for synthesized
//!   `number_text` rows. Modeled as enum [`CounterSource`].


use crate::dict::counters::subclasses::{CounterAge, CounterDaysKun, CounterDaysOn, CounterHalfhour, CounterHifumi, CounterMonths, CounterPeople, CounterTsu, CounterWari, NumberText};
use crate::dict::dao::KanaText;
use crate::dict::kani::{CounterArgs, CounterClass, SuffixKind};
use crate::dict::dao::KanjiText;
use crate::dict::split::hint_map::KANA_HINT_SPACE;
use crate::numbers::kana_form::{NumberToKanaOutput, number_to_kana};
use crate::numbers::kanji_form::{NotANumber, number_to_kanji, parse_number};
use crate::numbers::num_class::{DIGIT_KANJI_DEFAULT, POWER_KANJI};

#[derive(Debug, Clone)]
pub struct CounterText {
    pub text: String,
    pub kana: String,
    pub number_text: String,
    pub number: u64,
    pub source: Option<CounterSource>,
    pub ordinalp: bool,
    pub suffix: Option<String>,
    pub accepts_suffixes: Vec<SuffixKind>,
    pub suffix_descriptions: Vec<String>,
    pub digit_opts: Vec<DigitOptEntry>,
    pub common: Common,
    pub allowed: Vec<i32>,
    pub foreign: bool,
}

impl CounterText {
    /// Default `verify` method body — `dict-counters.lisp:31-36`,
    /// the `(:method (counter unique) ...)` clause on the gf:
    ///
    /// ```lisp
    /// (and (or (not (counter-allowed counter))
    ///          (find (number-value counter) (counter-allowed counter)))
    ///      unique)
    /// ```
    ///
    /// Empty `allowed` means "any number is fine"; otherwise the
    /// number must be a member. Subclass overrides
    /// ([`CounterTsu::verify`], [`CounterDaysOn::verify`]) chain back
    /// here through the dispatcher rather than calling this method
    /// directly — except [`CounterDaysOn::verify`], which mirrors
    /// the upstream `(call-next-method)` on the days-on method.
    ///
    /// [`CounterTsu::verify`]: crate::dict::counters::subclasses::CounterTsu::verify
    /// [`CounterDaysOn::verify`]: crate::dict::counters::subclasses::CounterDaysOn::verify
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.number as i64;
        let allowed_match = self.allowed.is_empty()
            || self.allowed.iter().any(|&a| a as i64 == n);
        allowed_match && unique
    }

    /// Default `value-string` body — `dict-counters.lisp:46-49`,
    /// the `(:method ((counter counter-text)) ...)` clause on the gf:
    ///
    /// ```lisp
    /// (format nil "Value: ~a~{ ~a~}"
    ///         (if (ordinalp counter) (ordinal-str value) value)
    ///         (reverse (counter-suffix-descriptions counter)))
    /// ```
    ///
    /// `~{ ~a~}` iterates the reversed descriptions list, prefixing
    /// each entry with a single space — output is
    /// `"Value: <n-or-ordinal>"` followed by zero or more
    /// `" <desc>"`. Subclass overrides
    /// ([`CounterHalfhour::value_string`], [`CounterMonths::value_string`],
    /// [`CounterWari::value_string`]) replace this body entirely;
    /// they're routed by [`crate::dict::counters::dispatchers::value_string`].
    ///
    /// [`CounterHalfhour::value_string`]: crate::dict::counters::subclasses::CounterHalfhour::value_string
    /// [`CounterMonths::value_string`]: crate::dict::counters::subclasses::CounterMonths::value_string
    /// [`CounterWari::value_string`]: crate::dict::counters::subclasses::CounterWari::value_string
    pub fn value_string(&self) -> String {
        let head = if self.ordinalp {
            crate::dict::counters::find_counter::ordinal_str(self.number as i64)
        } else {
            self.number.to_string()
        };
        let mut out = format!("Value: {}", head);
        for desc in self.suffix_descriptions.iter().rev() {
            out.push(' ');
            out.push_str(desc);
        }
        out
    }

    /// `get-kana` base primary — `dict-counters.lisp:64-67`:
    ///
    /// ```lisp
    /// (defmethod get-kana ((obj counter-text))
    ///   (counter-join obj n (number-to-kana n :separator *kana-hint-space*)
    ///                 (copy-seq (counter-kana obj))))
    /// ```
    ///
    /// Called by [`Counter::get_kana`] as the `call-next-method`
    /// target for subclasses whose specialized method returns
    /// `None`. Takes the wrapping [`Counter`] because the inner
    /// [`crate::dict::counters::cache::counter_join`] dispatches on
    /// subclass.
    pub fn primary_get_kana_for(counter: &Counter) -> String {
        let base = counter.base();
        let n = base.number;
        let number_kana = match number_to_kana(n, Some(KANA_HINT_SPACE), |x| {
            number_to_kanji(x, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
        }) {
            NumberToKanaOutput::Joined(s) => s,
            NumberToKanaOutput::Groups(_) => unreachable!(
                "number-to-kana with Some(separator) always returns Joined"
            ),
        };
        crate::dict::counters::cache::counter_join(
            counter,
            n as i64,
            number_kana,
            base.kana.clone(),
        )
    }

    /// Build the shared base from a recipe + the user-typed
    /// `number_text`, running the `initialize-instance :after`
    /// (`dict-counters.lisp:51`) that fills the `number` slot from
    /// `(parse-number number-text)`. Private so all construction
    /// flows through [`Counter::new`] — bypassing the dispatcher
    /// would skip subclass-specific slots (e.g. `digit_set`) and
    /// pick the wrong wrapper variant.
    fn from_args(args: &CounterArgs, number_text: String) -> Result<Self, NotANumber> {
        // dict-counters.lisp:51 (initialize-instance :after counter-text)
        let number = parse_number(&number_text)?;
        Ok(CounterText {
            text: args.text.clone(),
            kana: args.kana.clone(),
            number_text,
            number,
            source: args.source.clone(),
            ordinalp: args.ordinalp,
            suffix: args.suffix.clone(),
            accepts_suffixes: args.accepts.clone(),
            suffix_descriptions: args.suffix_descriptions.clone(),
            digit_opts: args.digit_opts.clone(),
            common: args.common,
            allowed: args.allowed.clone(),
            foreign: args.foreign,
        })
    }
}

#[derive(Debug, Clone)]
pub enum CounterSource {
    Kanji(KanjiText),
    Kana(KanaText),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Common {
    Inherit,
    Null,
    Score(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitOptEntry {
    pub key: DigitOptKey,
    pub ops: Vec<DigitOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitOptKey {
    Off,
    Digit(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigitOp {
    Geminate,
    Rendaku,
    Handakuten,
    Counter,
    Replace(String),
}

/// Family dispatcher for the counter-text class hierarchy. One
/// variant per Lisp class. Per-generic dispatch methods match-and-
/// delegate to the variant's own method on the wrapped struct, then
/// apply counter-text's `:around` wrappers (e.g. appending
/// [`CounterText::suffix`] to `get_kana` output) at the dispatcher
/// level.
#[derive(Debug, Clone)]
pub enum Counter {
    Base(CounterText),
    NumberText(NumberText),
    Age(CounterAge),
    DaysKun(CounterDaysKun),
    DaysOn(CounterDaysOn),
    Halfhour(CounterHalfhour),
    Hifumi(CounterHifumi),
    Months(CounterMonths),
    People(CounterPeople),
    Tsu(CounterTsu),
    Wari(CounterWari),
}

impl Counter {
    /// Mirrors `(apply 'make-instance '(,class ,@args :number-text ,number))`
    /// in `find-counter` (`dict-counters.lisp:278`). Single entry
    /// point: branches on [`CounterArgs::class`] to pick the wrapper
    /// variant, populates subclass-specific slots
    /// ([`CounterHifumi::digit_set`] from [`CounterArgs::digit_set`]),
    /// and runs the `counter-text` `initialize-instance :after` once
    /// via [`CounterText::from_args`] for every variant — same
    /// property as CLOS's inherited `:after`. The upstream catches
    /// `not-a-number` at the call site; here that surfaces as
    /// `Err(NotANumber)` for the caller to drop.
    ///
    /// [`CounterArgs::class`]: crate::dict::kani::CounterArgs#structfield.class
    /// [`CounterArgs::digit_set`]: crate::dict::kani::CounterArgs#structfield.digit_set
    /// [`CounterHifumi::digit_set`]: crate::dict::counters::subclasses::CounterHifumi#structfield.digit_set
    pub fn new(args: &CounterArgs, number_text: impl Into<String>) -> Result<Self, NotANumber> {
        // dict-counters.lisp:278 (find-counter — apply make-instance over recipe)
        let mut base = CounterText::from_args(args, number_text.into())?;
        Ok(match args.class {
            CounterClass::Text => Counter::Base(base),
            CounterClass::NumberText => Counter::NumberText(NumberText(base)),
            CounterClass::Tsu => Counter::Tsu(CounterTsu(base)),
            CounterClass::Age => Counter::Age(CounterAge(base)),
            CounterClass::DaysKun => {
                // dict-counters.lisp:687 (defclass counter-days-kun — :initform allowed)
                if base.allowed.is_empty() {
                    base.allowed = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 14, 20, 24, 30];
                }
                Counter::DaysKun(CounterDaysKun(base))
            }
            CounterClass::DaysOn => Counter::DaysOn(CounterDaysOn(base)),
            CounterClass::Halfhour => Counter::Halfhour(CounterHalfhour(base)),
            CounterClass::Months => {
                // dict-counters.lisp:722-723 (defclass counter-months — :initform allowed/digit-opts)
                if base.allowed.is_empty() {
                    base.allowed = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
                }
                if base.digit_opts.is_empty() {
                    base.digit_opts = vec![
                        DigitOptEntry {
                            key: DigitOptKey::Digit(4),
                            ops: vec![DigitOp::Replace("し".to_string())],
                        },
                        DigitOptEntry {
                            key: DigitOptKey::Digit(7),
                            ops: vec![DigitOp::Replace("しち".to_string())],
                        },
                        DigitOptEntry {
                            key: DigitOptKey::Digit(9),
                            ops: vec![DigitOp::Replace("く".to_string())],
                        },
                    ];
                }
                Counter::Months(CounterMonths(base))
            }
            CounterClass::People => Counter::People(CounterPeople(base)),
            CounterClass::Wari => Counter::Wari(CounterWari(base)),
            CounterClass::Hifumi => {
                // dict-counters.lisp:518-519 (defclass counter-hifumi) — :digit-set has no
                // :initform; upstream make-instance without :digit-set leaves the slot
                // unbound and any read raises UNBOUND-SLOT. Eager fail-fast here matches
                // that contract; a recipe missing :digit-set is a populator bug, not a
                // runtime input.
                assert!(
                    !args.digit_set.is_empty(),
                    "counter-hifumi requires non-empty :digit-set (dict-counters.lisp:518)"
                );
                Counter::Hifumi(CounterHifumi {
                    base,
                    digit_set: args.digit_set.clone(),
                })
            }
        })
    }

    /// Borrow the shared counter-text slot data underlying this
    /// variant. Used by dispatchers and by callers that need to read
    /// inherited slots (`text`, `kana`, `number`, `source`, etc.)
    /// without caring which subclass produced them.
    pub fn base(&self) -> &CounterText {
        match self {
            Counter::Base(c) => c,
            Counter::NumberText(c) => &c.0,
            Counter::Age(c) => &c.0,
            Counter::DaysKun(c) => &c.0,
            Counter::DaysOn(c) => &c.0,
            Counter::Halfhour(c) => &c.0,
            Counter::Hifumi(c) => &c.base,
            Counter::Months(c) => &c.0,
            Counter::People(c) => &c.0,
            Counter::Tsu(c) => &c.0,
            Counter::Wari(c) => &c.0,
        }
    }

    /// Family-level `get-kana` for the counter-text family.
    /// Implements the `:around` suffix-append wrapper
    /// (`dict-counters.lisp:69-71`):
    ///
    /// ```lisp
    /// (defmethod get-kana :around ((obj counter-text))
    ///   (let ((kana (call-next-method)))
    ///     (if (counter-suffix obj) (concatenate 'string kana (counter-suffix obj)) kana)))
    /// ```
    ///
    /// `call-next-method` dispatches to the subclass's specialized
    /// `get_kana` (if any returns `Some`); on `None` it falls back
    /// to [`CounterText::primary_get_kana_for`].
    pub fn get_kana(&self) -> String {
        let primary = match self {
            Counter::Tsu(c) => c.get_kana(),
            Counter::Hifumi(c) => c.get_kana(),
            Counter::DaysKun(c) => c.get_kana(),
            Counter::People(c) => c.get_kana(),
            Counter::Age(c) => c.get_kana(),
            Counter::NumberText(c) => Some(c.get_kana()),
            // Counter::Base / DaysOn / Halfhour / Months / Wari —
            // no specialized override; primary IS the base body.
            _ => None,
        };
        let body = primary.unwrap_or_else(|| CounterText::primary_get_kana_for(self));
        match &self.base().suffix {
            Some(suf) => format!("{}{}", body, suf),
            None => body,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Ground truth captured via `./ichiran-repl.sh` heredoc against the
    //! .103 ichiran install. Each test case mirrors a (number, counter)
    //! probe call to `find-counter`; the asserted slot values are read
    //! from the materialized counter-text instance(s) Lisp returned.
    //!
    //! Tests target `Counter::new`'s logic in isolation: the constructor
    //! takes a `CounterArgs` recipe + number-text and produces a
    //! `Counter` enum variant with the right slots populated. The
    //! upstream `find-counter` flow (lookup recipe by text key + verify)
    //! is not under test here — that lands with its own port.
    use super::*;
    use crate::dict::kani::CounterArgs;

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
        assert_eq!(base.digit_opts[0].ops, vec![DigitOp::Replace("し".to_string())]);
        assert_eq!(base.digit_opts[1].key, DigitOptKey::Digit(7));
        assert_eq!(base.digit_opts[1].ops, vec![DigitOp::Replace("しち".to_string())]);
        assert_eq!(base.digit_opts[2].key, DigitOptKey::Digit(9));
        assert_eq!(base.digit_opts[2].ops, vec![DigitOp::Replace("く".to_string())]);
        assert_eq!(base.number, 4);
    }

    #[test]
    fn hifumi_propagates_digit_set_from_recipe() {
        // dict-counters.lisp:541 — (args 'counter-hifumi "株" "かぶ" :digit-set '(1 2)).
        // Lisp probe of (find-counter "1" "株") returns COUNTER-HIFUMI with
        // digit-set=(1 2) populated from the :digit-set initarg.
        let args =
            CounterArgs::new(CounterClass::Hifumi, "株", "かぶ").digit_set(vec![1, 2]);
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

}
