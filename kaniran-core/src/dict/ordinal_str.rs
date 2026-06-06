//! Port of `ichiran/dict:ordinal-str` (`dict-counters.lisp:38`).
//!
//! Renders `n` as an English ordinal — "1st", "22nd", "113th", etc.

pub fn ordinal_str(n: i64) -> String {
    let digit = n.rem_euclid(10);
    let teen = (11..=19).contains(&n.rem_euclid(100));
    let suffix = if teen {
        "th"
    } else {
        match digit {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{}{}", n, suffix)
}

#[cfg(test)]
mod tests {
    //! Unit tests cover the three teen-band edges plus a sample of
    //! each digit-suffix case. Bulk behavioural coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/ordinal_str.parquet`
    //! (134 rows) replayed by `audit_fixtures`.
    use super::*;

    #[test]
    fn small_digits_select_st_nd_rd_th() {
        assert_eq!(ordinal_str(1), "1st");
        assert_eq!(ordinal_str(2), "2nd");
        assert_eq!(ordinal_str(3), "3rd");
        assert_eq!(ordinal_str(4), "4th");
        assert_eq!(ordinal_str(7), "7th");
        assert_eq!(ordinal_str(0), "0th");
    }

    #[test]
    fn teens_force_th() {
        // (mod n 100) ∈ 11..=19 → "th" regardless of last digit.
        assert_eq!(ordinal_str(11), "11th");
        assert_eq!(ordinal_str(12), "12th");
        assert_eq!(ordinal_str(13), "13th");
        assert_eq!(ordinal_str(19), "19th");
        assert_eq!(ordinal_str(111), "111th");
        assert_eq!(ordinal_str(212), "212th");
    }

    #[test]
    fn non_teen_twos_threes_etc_use_digit_suffix() {
        assert_eq!(ordinal_str(21), "21st");
        assert_eq!(ordinal_str(22), "22nd");
        assert_eq!(ordinal_str(23), "23rd");
        assert_eq!(ordinal_str(101), "101st");
        assert_eq!(ordinal_str(122), "122nd");
        assert_eq!(ordinal_str(1000), "1000th");
    }

    #[test]
    fn negative_uses_floor_mod() {
        // Lisp (mod -1 10) = 9, so digit "9" → "th". Format prints "-1".
        assert_eq!(ordinal_str(-1), "-1th");
        // (mod -21 10) = 9; (mod -21 100) = 79 (not in 11..=19) → "th".
        assert_eq!(ordinal_str(-21), "-21th");
    }
}
