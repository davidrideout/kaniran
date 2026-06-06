//! Port of `ichiran/dict:verify` (`dict-counters.lisp:31`).
//!
//! Tells `find-counter` whether a freshly-constructed [`Counter`]
//! recipe is a real match for the queried number string, dispatching
//! to the counter-text default or a tsu / days-on override.

use crate::dict::counter_text_class::Counter;

pub fn verify(counter: &Counter, unique: bool) -> bool {
    match counter {
        Counter::Tsu(c) => c.verify(unique),
        Counter::DaysOn(c) => c.verify(unique),
        // Every other variant inherits the (T T) default method.
        Counter::Base(c)
        | Counter::NumberText(crate::dict::number_text_class::NumberText(c))
        | Counter::Age(crate::dict::counter_age_class::CounterAge(c))
        | Counter::DaysKun(crate::dict::counter_days_kun_class::CounterDaysKun(c))
        | Counter::Halfhour(crate::dict::counter_halfhour_class::CounterHalfhour(c))
        | Counter::Months(crate::dict::counter_months_class::CounterMonths(c))
        | Counter::People(crate::dict::counter_people_class::CounterPeople(c))
        | Counter::Wari(crate::dict::counter_wari_class::CounterWari(c)) => c.verify(unique),
        Counter::Hifumi(c) => c.base.verify(unique),
    }
}

#[cfg(test)]
mod tests {
    //! Unit coverage targets the three dispatch arms (Tsu, DaysOn,
    //! default) at the boundary values that distinguish them. Bulk
    //! behavioural coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/verify.parquet`
    //! (137,676 rows across 11 variants) replayed by `audit_fixtures`.
    use super::*;
    use crate::dict::counter_days_on_class::CounterDaysOn;
    use crate::dict::counter_text_class::{Common, Counter, CounterText};
    use crate::dict::counter_tsu_class::CounterTsu;

    fn base(number: u64, allowed: Vec<i32>) -> CounterText {
        CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: number.to_string(),
            number,
            source: None,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed,
            foreign: false,
        }
    }

    #[test]
    fn default_passes_when_allowed_empty() {
        let c = Counter::Base(base(0, vec![]));
        assert!(verify(&c, true));
        assert!(!verify(&c, false));
    }

    #[test]
    fn default_checks_allowed_membership() {
        let c = Counter::Base(base(5, vec![1, 2, 3, 4, 5]));
        assert!(verify(&c, true));
        let c = Counter::Base(base(6, vec![1, 2, 3, 4, 5]));
        assert!(!verify(&c, true));
    }

    #[test]
    fn tsu_in_range() {
        for n in 1..=9 {
            assert!(verify(&Counter::Tsu(CounterTsu(base(n, vec![]))), true), "n={}", n);
        }
        assert!(!verify(&Counter::Tsu(CounterTsu(base(0, vec![]))), true));
        assert!(!verify(&Counter::Tsu(CounterTsu(base(10, vec![]))), true));
    }

    #[test]
    fn days_on_excludes_20_and_2_through_10() {
        // Valid: n == 1 or n > 10, but not 20.
        assert!(verify(&Counter::DaysOn(CounterDaysOn(base(1, vec![]))), true));
        assert!(verify(&Counter::DaysOn(CounterDaysOn(base(11, vec![]))), true));
        assert!(verify(&Counter::DaysOn(CounterDaysOn(base(31, vec![]))), true));
        // Boundary: 20 is owned by counter-days-kun.
        assert!(!verify(&Counter::DaysOn(CounterDaysOn(base(20, vec![]))), true));
        // 2..=10 (except 1) are kun-yomi territory.
        for n in 2..=10 {
            assert!(!verify(&Counter::DaysOn(CounterDaysOn(base(n, vec![]))), true), "n={}", n);
        }
    }

    #[test]
    fn days_on_chains_to_default_for_allowed() {
        // call-next-method runs the default; if allowed is set and n
        // doesn't match, the chain returns false even when the days-on
        // gate passed. allowed=NIL is the captured shape; this case
        // pins the behaviour for any future days-on recipe with a list.
        let c = Counter::DaysOn(CounterDaysOn(base(11, vec![1, 11, 31])));
        assert!(verify(&c, true));
        let c = Counter::DaysOn(CounterDaysOn(base(11, vec![1, 31])));
        assert!(!verify(&c, true));
    }

    #[test]
    fn unique_false_overrides_everything() {
        // Default + Tsu + DaysOn all AND with `unique`; unique=false
        // short-circuits.
        assert!(!verify(&Counter::Base(base(0, vec![])), false));
        assert!(!verify(&Counter::Tsu(CounterTsu(base(5, vec![]))), false));
        assert!(!verify(&Counter::DaysOn(CounterDaysOn(base(11, vec![]))), false));
    }
}
