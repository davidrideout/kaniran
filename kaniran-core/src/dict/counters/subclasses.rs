//! Port of the dict-counters.lisp counter subclasses — small structs
//! that extend [`super::classes::CounterText`] with class-specific
//! slot defaults and method overrides (`get-kana`, `value-string`,
//! `verify`). Section order follows upstream's class definitions in
//! dict-counters.lisp.


use crate::dict::counters::classes::CounterText;
use crate::dict::split::hint_map::KANA_HINT_SPACE;
use crate::numbers::kana_form::{NumberToKanaOutput, number_to_kana};
use crate::numbers::kanji_form::number_to_kanji;
use crate::numbers::num_class::{DIGIT_KANJI_DEFAULT, POWER_KANJI};

// ----- was number_text_class.rs -----

#[derive(Debug, Clone)]
pub struct NumberText(pub CounterText);

impl NumberText {
    /// `get-kana` override — `dict-counters.lisp:208-209`:
    /// `(number-to-kana (number-value obj) :separator *kana-hint-space*)`.
    /// Always specializes; no call-next-method path.
    pub fn get_kana(&self) -> String {
        match number_to_kana(self.0.number, Some(KANA_HINT_SPACE), |x| {
            number_to_kanji(x, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
        }) {
            NumberToKanaOutput::Joined(s) => s,
            NumberToKanaOutput::Groups(_) => unreachable!(
                "number-to-kana with Some(separator) always returns Joined"
            ),
        }
    }
}


// ----- was counter_halfhour_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterHalfhour(pub CounterText);

impl CounterHalfhour {
    /// `value-string` override — `dict-counters.lisp:393-394`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-halfhour))
    ///   (format nil "~a:30" (number-value counter)))
    /// ```
    pub fn value_string(&self) -> String {
        format!("{}:30", self.0.number)
    }
}


// ----- was counter_tsu_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterTsu(pub CounterText);

impl CounterTsu {
    /// `dict-counters.lisp:499` — `(<= 1 (number-value counter) 9)`
    /// AND `unique`. Ignores the `allowed` slot entirely; the bare つ
    /// counter is valid only for the kun-yomi 1..9 range and the
    /// `get-kana` table covers exactly those values.
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.0.number;
        (1..=9).contains(&n) && unique
    }

    /// `get-kana` override — `dict-counters.lisp:502-513`.
    /// Closed kun-yomi table for 1..9; everything else falls
    /// through to `call-next-method` (the counter-text base
    /// primary). Returns `None` to signal fall-through.
    pub fn get_kana(&self) -> Option<String> {
        let n = self.0.number as i64;
        Some(match n {
            1 => "ひとつ".to_string(),
            2 => "ふたつ".to_string(),
            3 => "みっつ".to_string(),
            4 => "よっつ".to_string(),
            5 => "いつつ".to_string(),
            6 => "むっつ".to_string(),
            7 => "ななつ".to_string(),
            8 => "やっつ".to_string(),
            9 => "ここのつ".to_string(),
            _ => return None,
        })
    }
}


// ----- was counter_hifumi_class.rs -----

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
mod counter_hifumi_class_tests {
    use super::*;
    use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};

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


// ----- was counter_days_kun_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterDaysKun(pub CounterText);

impl CounterDaysKun {
    /// `get-kana` override — `dict-counters.lisp:689-704`.
    /// Closed table over the allowed kun-yomi day-count values.
    /// Returns `Some` for every table entry; `Some(String::new())`
    /// for off-table values, mirroring upstream `case` without a
    /// `t` clause returning nil — which the `:around` then
    /// concatenates with the suffix as empty. Never falls through
    /// to `call-next-method` because the `verify` restriction
    /// limits inputs to the table entries.
    pub fn get_kana(&self) -> Option<String> {
        Some(match self.0.number as i64 {
            1 => "ついたち".to_string(),
            2 => "ふつか".to_string(),
            3 => "みっか".to_string(),
            4 => "よっか".to_string(),
            5 => "いつか".to_string(),
            6 => "むいか".to_string(),
            7 => "なのか".to_string(),
            8 => "ようか".to_string(),
            9 => "ここのか".to_string(),
            10 => "とうか".to_string(),
            14 => "じゅうよっか".to_string(),
            20 => "はつか".to_string(),
            24 => "にじゅうよっか".to_string(),
            30 => "みそか".to_string(),
            // `case` without `t` returns nil upstream;
            // `(concatenate 'string nil suffix)` treats nil as
            // empty. Mirror by emitting the empty string here.
            _ => String::new(),
        })
    }
}


// ----- was counter_days_on_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterDaysOn(pub CounterText);

impl CounterDaysOn {
    /// `dict-counters.lisp:711-716`:
    ///
    /// ```lisp
    /// (and (or (> n 10) (= n 1))
    ///      (/= n 20)
    ///      (call-next-method))
    /// ```
    ///
    /// `n` is the counter's number-value. The on-yomi day-count にち
    /// is valid for the literal day "1日" or any value above ten,
    /// excluding 20 (which is owned by [`CounterDaysKun`]).
    /// `(call-next-method)` falls through to [`CounterText::verify`].
    ///
    /// [`CounterDaysKun`]: crate::dict::counters::subclasses::CounterDaysKun
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.0.number;
        (n > 10 || n == 1) && n != 20 && self.0.verify(unique)
    }
}


// ----- was counter_months_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterMonths(pub CounterText);

const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March",
    "April", "May", "June",
    "July", "August", "September",
    "October", "November", "December",
];

impl CounterMonths {
    /// `value-string` override — `dict-counters.lisp:725-730`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-months))
    ///   (aref #("January" ... "December") (1- (number-value counter))))
    /// ```
    ///
    /// Upstream `aref` raises a bounds error when `number-value` is
    /// outside `1..=12`; here that surfaces as a panic from the array
    /// index. Counter `verify` keeps `number` inside `allowed`
    /// (`[1..=12]` for this class) so the call site never reaches this
    /// method with an out-of-range value.
    pub fn value_string(&self) -> String {
        MONTH_NAMES[(self.0.number - 1) as usize].to_string()
    }
}


// ----- was counter_people_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterPeople(pub CounterText);

impl CounterPeople {
    /// `get-kana` override — `dict-counters.lisp:737-741`. Returns
    /// ひとり for 1, ふたり for 2, otherwise `None` to fall through
    /// to the counter-text base primary.
    pub fn get_kana(&self) -> Option<String> {
        match self.0.number as i64 {
            1 => Some("ひとり".to_string()),
            2 => Some("ふたり".to_string()),
            _ => None,
        }
    }
}


// ----- was counter_wari_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterWari(pub CounterText);

impl CounterWari {
    /// `value-string` override — `dict-counters.lisp:748-749`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-wari))
    ///   (format nil "~a%" (* 10 (number-value counter))))
    /// ```
    pub fn value_string(&self) -> String {
        format!("{}%", self.0.number * 10)
    }
}


// ----- was counter_age_class.rs -----

#[derive(Debug, Clone)]
pub struct CounterAge(pub CounterText);

impl CounterAge {
    /// `get-kana` override — `dict-counters.lisp:759-762`. Returns
    /// はたち for 20, otherwise `None` to fall through to the
    /// counter-text base primary.
    pub fn get_kana(&self) -> Option<String> {
        match self.0.number as i64 {
            20 => Some("はたち".to_string()),
            _ => None,
        }
    }
}
