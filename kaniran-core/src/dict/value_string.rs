//! Port of `ichiran/dict:value-string` (gf — `dict-counters.lisp:44-49`).
//!
//! Renders a counter's numeric value as a human-readable display
//! string, dispatching to the counter-text default or a halfhour /
//! months / wari override.

use crate::dict::counter_text_class::Counter;

pub fn value_string(counter: &Counter) -> String {
    match counter {
        Counter::Halfhour(c) => c.value_string(),
        Counter::Months(c) => c.value_string(),
        Counter::Wari(c) => c.value_string(),
        // Every other variant inherits the (counter-text) default.
        Counter::Base(c)
        | Counter::NumberText(crate::dict::number_text_class::NumberText(c))
        | Counter::Age(crate::dict::counter_age_class::CounterAge(c))
        | Counter::DaysKun(crate::dict::counter_days_kun_class::CounterDaysKun(c))
        | Counter::DaysOn(crate::dict::counter_days_on_class::CounterDaysOn(c))
        | Counter::People(crate::dict::counter_people_class::CounterPeople(c))
        | Counter::Tsu(crate::dict::counter_tsu_class::CounterTsu(c)) => c.value_string(),
        Counter::Hifumi(c) => c.base.value_string(),
    }
}

#[cfg(test)]
mod tests {
    //! Unit coverage targets the four dispatch arms at the boundaries
    //! that distinguish them. Bulk behavioural coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/value_string.parquet`
    //! replayed by `audit_fixtures`.
    use super::*;
    use crate::dict::counter_halfhour_class::CounterHalfhour;
    use crate::dict::counter_months_class::CounterMonths;
    use crate::dict::counter_text_class::{Common, Counter, CounterText};
    use crate::dict::counter_wari_class::CounterWari;

    fn base(number: u64, ordinalp: bool, descs: Vec<&str>) -> CounterText {
        CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: number.to_string(),
            number,
            source: None,
            ordinalp,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: descs.into_iter().map(String::from).collect(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign: false,
        }
    }

    #[test]
    fn default_numeric() {
        let c = Counter::Base(base(5, false, vec![]));
        assert_eq!(value_string(&c), "Value: 5");
    }

    #[test]
    fn default_ordinal() {
        let c = Counter::Base(base(2, true, vec![]));
        assert_eq!(value_string(&c), "Value: 2nd");
    }

    #[test]
    fn default_with_descriptions_reversed_and_space_prefixed() {
        // Lisp `~{ ~a~}` over (reverse '("d1" "d2")) → " d2 d1".
        let c = Counter::Base(base(5, false, vec!["d1", "d2"]));
        assert_eq!(value_string(&c), "Value: 5 d2 d1");
    }

    #[test]
    fn halfhour_emits_n_colon_30() {
        let c = Counter::Halfhour(CounterHalfhour(base(5, false, vec![])));
        assert_eq!(value_string(&c), "5:30");
    }

    #[test]
    fn months_indexes_into_english_array() {
        let c = Counter::Months(CounterMonths(base(1, false, vec![])));
        assert_eq!(value_string(&c), "January");
        let c = Counter::Months(CounterMonths(base(3, false, vec![])));
        assert_eq!(value_string(&c), "March");
        let c = Counter::Months(CounterMonths(base(12, false, vec![])));
        assert_eq!(value_string(&c), "December");
    }

    #[test]
    fn wari_emits_n_times_10_percent() {
        let c = Counter::Wari(CounterWari(base(5, false, vec![])));
        assert_eq!(value_string(&c), "50%");
        let c = Counter::Wari(CounterWari(base(1, false, vec![])));
        assert_eq!(value_string(&c), "10%");
    }
}
