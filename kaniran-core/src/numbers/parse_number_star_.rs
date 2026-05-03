//! Port of `ichiran/numbers:parse-number*` (`numbers.lisp:57`).
//!
//! Recursive parser over a slice of pre-classified numeric atoms.
//! Finds the largest `(NumClass::P, exponent)` token in the slice and
//! splits around it: `left * 10^exponent + right`. If no power token is
//! present, the slice is treated as a sequence of digits and reduced
//! left-to-right (`a, b, c → a*100 + b*10 + c`).
//!
//! Diverges from the Lisp's `&key (start 0) (end (length na))` by
//! taking a `&[(NumClass, u8)]` slice — Rust callers slice naturally
//! without explicit start/end indices.

use super::kani_num_class::NumClass;

pub fn parse_number_star_(na: &[(NumClass, u8)]) -> u64 {
    let mut mp: u8 = 0;
    let mut mi: Option<usize> = None;
    for (i, &(class, val)) in na.iter().enumerate() {
        if class == NumClass::P && val > mp {
            mp = val;
            mi = Some(i);
        }
    }
    match mi {
        None => na
            .iter()
            .fold(0u64, |a, &(_class, v)| a * 10 + v as u64),
        Some(idx) if idx == 0 => {
            let head = 10u64.pow(mp as u32);
            let tail = if na.len() > 1 {
                parse_number_star_(&na[1..])
            } else {
                0
            };
            head + tail
        }
        Some(idx) => {
            let left = parse_number_star_(&na[..idx]);
            let right = if idx + 1 < na.len() {
                parse_number_star_(&na[idx + 1..])
            } else {
                0
            };
            left * 10u64.pow(mp as u32) + right
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NumClass::*;

    #[test]
    fn pure_digits_reduce_left_to_right() {
        // 二〇二〇 — four jd digits, no power → 2020.
        let na = &[(Jd, 2), (Jd, 0), (Jd, 2), (Jd, 0)];
        assert_eq!(parse_number_star_(na), 2020);
    }

    #[test]
    fn lone_power_at_start_yields_power_value() {
        // 万 alone → 10000.
        let na = &[(P, 4)];
        assert_eq!(parse_number_star_(na), 10000);
    }

    #[test]
    fn power_with_following_remainder_adds() {
        // 千五百 — split on 千 (3): left=∅ → +1000; right=五百 → 500. = 1500.
        let na = &[(P, 3), (Jd, 5), (P, 2)];
        assert_eq!(parse_number_star_(na), 1500);
    }

    #[test]
    fn nested_split_with_left_and_right() {
        // 千二百三十四 = 1234. 千(3) is largest power → left=∅+1000, right=二百三十四=234. → 1234.
        let na = &[(P, 3), (Jd, 2), (P, 2), (Jd, 3), (P, 1), (Jd, 4)];
        assert_eq!(parse_number_star_(na), 1234);
    }
}
