use super::constants::DIGIT_TO_KANA;
use super::constants::POWER_TO_KANA;
use super::helpers::char_number_class_hash;
use super::kani_num_class::NumClass;
use crate::characters::voicing::geminate;
use crate::characters::text::join;
use crate::characters::voicing::{rendaku, Voicing};

/// Port of `ichiran/numbers:num-sandhi` (`numbers.lisp:82-112`).
///
/// Apply euphonic sound changes between two adjacent number kana
/// groups. Geminates the trailing mora of `s1` (e.g. `いち → いっ`) and
/// voices the leading mora of `s2` (e.g. `せん → ぜん`, `ひゃく →
/// びゃく`) when the `(c1, v1, c2, v2)` quadruple matches one of the
/// upstream specializers; otherwise the strings concatenate unchanged.
/// `c1`/`v1` are `Option`-typed for the "no previous group" startup
/// state.
pub fn num_sandhi(
    c1: Option<NumClass>,
    v1: Option<u8>,
    c2: NumClass,
    v2: u8,
    s1: &str,
    s2: &str,
) -> String {
    use NumClass::*;
    let mut s1_buf = s1.to_string();
    let mut s2_buf = s2.to_string();
    match (c1, v1, c2, v2) {
        (Some(Jd), Some(1), P, v) if matches!(v, 3 | 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(Jd), Some(3), P, v) if matches!(v, 2 | 3) => {
            rendaku(&mut s2_buf, Voicing::Dakuten);
        }
        (Some(Jd), Some(6), P, 2) => {
            geminate(&mut s1_buf);
            rendaku(&mut s2_buf, Voicing::Handakuten);
        }
        (Some(Jd), Some(6), P, 16) => {
            geminate(&mut s1_buf);
        }
        (Some(Jd), Some(8), P, 2) => {
            geminate(&mut s1_buf);
            rendaku(&mut s2_buf, Voicing::Handakuten);
        }
        (Some(Jd), Some(8), P, v) if matches!(v, 3 | 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(P), Some(1), P, v) if matches!(v, 12 | 16) => {
            geminate(&mut s1_buf);
        }
        (Some(P), Some(2), P, 16) => {
            geminate(&mut s1_buf);
        }
        _ => {}
    }
    format!("{s1_buf}{s2_buf}")
}

/// Port of `ichiran/numbers:group-to-kana` (`numbers.lisp:114`).
///
/// Convert one number group (a sequence of `(NumClass, u8)` pairs that
/// represent a single sub-number such as `三百` = `(Jd 3)(P 2)`) into
/// its hiragana reading. Walks the group left-to-right, looking up
/// each pair's reading in the supplied tables and folding via
/// [`super::kana_form::num_sandhi`] to apply euphonic changes between
/// adjacent pairs.
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

/// Port of `ichiran/numbers:number-to-kana` (`numbers.lisp:122`).
///
/// Render an integer as its Japanese reading: integer → kanji string
/// (via `kanji_method`) → split into number groups (each ending in a
/// strictly-larger power-of-ten than its interior) → render each group
/// via [`super::kana_form::group_to_kana`] → either join with
/// `separator` ([`NumberToKanaOutput::Joined`]) or return the per-group
/// readings ([`NumberToKanaOutput::Groups`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberToKanaOutput {
    Joined(String),
    Groups(Vec<String>),
}

pub fn number_to_kana(
    n: u64,
    separator: Option<char>,
    kanji_method: impl Fn(u64) -> String,
) -> NumberToKanaOutput {
    let h = char_number_class_hash();
    let mut groups: Vec<Vec<(NumClass, u8)>> = Vec::new();
    let mut cur: Vec<(NumClass, u8)> = Vec::new();
    let mut last: Option<(NumClass, u8)> = None;

    for kanji in kanji_method(n).chars() {
        let &(class, val) = h
            .get(&kanji)
            .expect("kanji_method emitted a character not in *char-number-class-hash*");
        let extend = match last {
            None => true,
            Some((last_class, last_val)) => {
                class == NumClass::P
                    && (last_class == NumClass::Jd || (last_class == NumClass::P && val > last_val))
            }
        };
        if !extend {
            groups.push(std::mem::take(&mut cur));
        }
        cur.push((class, val));
        last = Some((class, val));
    }
    groups.push(cur);

    let parts: Vec<String> = groups
        .iter()
        .map(|g| group_to_kana(g, DIGIT_TO_KANA, POWER_TO_KANA))
        .collect();
    match separator {
        Some(sep) => {
            let mut sep_buf = [0u8; 4];
            let sep_str = sep.encode_utf8(&mut sep_buf);
            NumberToKanaOutput::Joined(join(sep_str, &parts))
        }
        None => NumberToKanaOutput::Groups(parts),
    }
}

#[cfg(test)]
mod tests;
