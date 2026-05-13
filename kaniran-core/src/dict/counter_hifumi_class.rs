//! Port of `ichiran/dict:counter-hifumi` (`dict-counters.lisp:518`).
//!
//! Counter cache entry for the ~30 counters that take native
//! kun-yomi numeric prefixes (ひと, ふた, み, よ, いつ, む, なな,
//! や, ここの, と) for small counts instead of the Sino-Japanese
//! number readings. Adds one slot over [`CounterText`]:
//!
//! - `digit_set` — set of digit values for which the kun-yomi prefix
//!   applies. Values outside the set fall through to the parent's
//!   default reading.
//!
//! The Lisp class declares the slot with no default; every observed
//! `def-special-counter` callsite supplies `:digit-set` explicitly,
//! so the field is required at construction time. Most call sites
//! pass `[1, 2]` or `[1, 2, 3]`; the largest seen is `[1, 2, 3, 4, 5]`
//! (棹/竿).
//!
//! The `get-kana` override (table-driven prefix substitution) ports
//! alongside `find-counter` in a later wave.

use crate::dict::counter_text_class::CounterText;

#[derive(Debug, Clone)]
pub struct CounterHifumi {
    pub base: CounterText,
    pub digit_set: Vec<i32>,
}

impl CounterHifumi {
    /// `get-kana` override — `dict-counters.lisp:521-538`.
    ///
    /// ```lisp
    /// (defmethod get-kana ((obj counter-hifumi))
    ///   (cond ((find (number-value obj) (digit-set obj))
    ///          (concatenate 'string
    ///                       (case (number-value obj)
    ///                         (1 "ひと") (2 "ふた") (3 "み") (4 "よ")
    ///                         (5 "いつ") (6 "む") (7 "なな") (8 "や")
    ///                         (9 "ここの") (10 "と"))
    ///                       (counter-kana obj)))
    ///         (t (call-next-method))))
    /// ```
    ///
    /// Returns:
    /// - `Some(prefix + counter_kana)` when `value` is in `digit_set`.
    ///   The inner `case` without a `t` clause returns `nil` for
    ///   values outside 1..=10 — `(concatenate 'string nil ...)`
    ///   treats nil as empty, so the result is just `counter_kana`.
    ///   This is NOT `call-next-method` and must not fall through.
    /// - `None` when `value` is NOT in `digit_set` — upstream's
    ///   `(t (call-next-method))` arm.
    pub fn get_kana(&self) -> Option<String> {
        let value = self.base.number as i64;
        if !self.digit_set.iter().any(|&d| i64::from(d) == value) {
            // outside digit-set → call-next-method
            return None;
        }
        let prefix = match value {
            1 => "ひと",
            2 => "ふた",
            3 => "み",
            4 => "よ",
            5 => "いつ",
            6 => "む",
            7 => "なな",
            8 => "や",
            9 => "ここの",
            10 => "と",
            _ => "",
        };
        Some(format!("{}{}", prefix, self.base.kana))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::counter_text_class::{Common, Counter, CounterSource, CounterText};

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
        assert!(result.contains("かぶ"), "expected counter-kana, got {:?}", result);
    }
}
