//! Port of `ichiran/numbers:group-to-kana` (`numbers.lisp:114`).
//!
//! Convert one number group (a sequence of `(NumClass, u8)` pairs that
//! represent a single sub-number such as `三百` = `(Jd 3)(P 2)`) into
//! its hiragana reading. Walks the group left-to-right, looking up
//! each pair's reading in the supplied tables and folding via
//! [`super::num_sandhi::num_sandhi`] to apply euphonic changes between
//! adjacent pairs.
//!
//! Diverges from the Lisp's `&key class-to-kana` plist (`(:jd ,table
//! :p ,table)`) by taking two explicit `&[&str]` / `&[(u8, &str)]`
//! parameters — Rust callers can swap either table independently. Top-
//! level callers pass [`super::constants::DIGIT_TO_KANA`]
//! and [`super::constants::POWER_TO_KANA`].
//!
//! [`NumClass::Ad`] is mapped to the supplied `digit_table` (both
//! `Jd` and `Ad` carry `0..=9` values); upstream this would crash
//! because the plist has no `:ad` key — Rust handles it gracefully
//! because the two share the table shape.

use super::kani_num_class::NumClass;
use super::num_sandhi::num_sandhi;

pub fn group_to_kana(
    group: &[(NumClass, u8)],
    digit_table: &[&str],
    power_table: &[(u8, &str)],
) -> String {
    let mut result = String::new();
    let mut prev_class: Option<NumClass> = None;
    let mut prev_val: Option<u8> = None;
    for &(class, val) in group {
        let kana = lookup(class, val, digit_table, power_table);
        result = num_sandhi(prev_class, prev_val, class, val, &result, kana);
        prev_class = Some(class);
        prev_val = Some(val);
    }
    result
}

fn lookup<'a>(
    class: NumClass,
    val: u8,
    digit_table: &'a [&'a str],
    power_table: &'a [(u8, &'a str)],
) -> &'a str {
    match class {
        NumClass::Jd | NumClass::Ad => digit_table[val as usize],
        NumClass::P => power_table
            .iter()
            .find_map(|&(k, s)| if k == val { Some(s) } else { None })
            .expect("power_table missing entry for exponent"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::constants::{DIGIT_TO_KANA, POWER_TO_KANA};
    use super::*;
    use NumClass::*;

    fn g(group: &[(NumClass, u8)]) -> String {
        group_to_kana(group, DIGIT_TO_KANA, POWER_TO_KANA)
    }

    #[test]
    fn single_digit_group() {
        assert_eq!(g(&[(Jd, 4)]), "よん");
    }

    #[test]
    fn lone_power_group() {
        assert_eq!(g(&[(P, 3)]), "せん");
    }

    #[test]
    fn digit_then_power_with_sandhi() {
        // [(Jd 3)(P 2)] = 三百 → "さん" + rendaku("ひゃく") = "さんびゃく"
        assert_eq!(g(&[(Jd, 3), (P, 2)]), "さんびゃく");
    }

    #[test]
    fn digit_then_power_no_sandhi() {
        // [(Jd 2)(P 2)] = 二百 → "に" + "ひゃく" (no transformation)
        assert_eq!(g(&[(Jd, 2), (P, 2)]), "にひゃく");
    }

    #[test]
    fn caller_can_swap_digit_table() {
        // Stand-in alt table with different readings — exercises the
        // `class-to-kana` swap path that the Lisp keyword enables.
        const ALT: &[&str] = &["zero", "one", "two", "three", "four",
                               "five", "six", "seven", "eight", "nine"];
        assert_eq!(group_to_kana(&[(Jd, 5)], ALT, POWER_TO_KANA), "five");
    }
}
