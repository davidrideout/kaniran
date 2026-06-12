use super::helpers::char_number_class_hash;
use super::kani_num_class::NumClass;
use thiserror::Error;

/// Port of `ichiran/numbers:not-a-number` (`numbers.lisp:67`).
///
/// Error raised by [`super::kanji_form::parse_number`] when its input
/// string contains a character that isn't a recognized numeric glyph.
/// Carries the offending input and a free-form reason string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{text:?} is not a number: {reason}")]
pub struct NotANumber {
    pub text: String,
    pub reason: String,
}

/// Port of `ichiran/numbers:number-to-kanji` (`numbers.lisp:35`).
///
/// Render a non-negative integer as a kanji number string —
/// `1234` → `千二百三十四`, `0` → `〇`. The `digits` and `powers`
/// parameters are the per-digit and per-power glyph tables; `one_sen`
/// is the recursion flag controlling whether a leading `一` is
/// suppressed before `千` (`one_sen = false`) or only before `百`
/// (`one_sen = true`).
pub fn number_to_kanji(n: u128, digits: &str, powers: &str, one_sen: bool) -> String {
    let digit_chars: Vec<char> = digits.chars().collect();
    let power_chars: Vec<char> = powers.chars().collect();
    if n == 0 {
        return digit_chars[0].to_string();
    }
    let mut mp: u128 = 1;
    let mut mc: char = power_chars[0];
    let mut p: u128 = 1;
    for &c in &power_chars {
        if p > n {
            break;
        }
        if c != ' ' {
            mp = p;
            mc = c;
        }
        match p.checked_mul(10) {
            Some(np) => p = np,
            None => break,
        }
    }
    if mp == 1 {
        return digit_chars[n as usize].to_string();
    }
    let qt = n / mp;
    let rem = n % mp;
    let head_threshold: u128 = if one_sen { 100 } else { 1000 };
    let head = if qt == 1 && mp <= head_threshold {
        String::new()
    } else {
        number_to_kanji(qt, digits, powers, true)
    };
    let tail = if rem == 0 {
        String::new()
    } else {
        number_to_kanji(rem, digits, powers, one_sen)
    };
    format!("{head}{mc}{tail}")
}

/// Port of `ichiran/numbers:parse-number*` (`numbers.lisp:57`).
///
/// Recursive parser over a slice of pre-classified numeric atoms.
/// Finds the largest `(NumClass::P, exponent)` token in the slice and
/// splits around it: `left * 10^exponent + right`. If no power token is
/// present, the slice is treated as a sequence of digits and reduced
/// left-to-right (`a, b, c → a*100 + b*10 + c`).
///
/// The Lisp computes in bignums; this port computes in `u128` and
/// returns `None` past its range (a ~39-digit value — the segmenter's
/// 20-char number gate at `dict.lisp:1095-1098` keeps reachable values
/// at or below 10^35, so `None` can only occur through the ungated
/// direct-lookup paths).
pub fn parse_number_star_(na: &[(NumClass, u8)]) -> Option<u128> {
    let mut mp: u8 = 0;
    let mut mi: Option<usize> = None;
    for (i, &(class, val)) in na.iter().enumerate() {
        if class == NumClass::P && val > mp {
            mp = val;
            mi = Some(i);
        }
    }
    match mi {
        None => na.iter().try_fold(0u128, |acc, &(_class, v)| {
            acc.checked_mul(10)?.checked_add(v as u128)
        }),
        Some(idx) if idx == 0 => {
            let head = 10u128.pow(mp as u32);
            let tail = if na.len() > 1 {
                parse_number_star_(&na[1..])?
            } else {
                0
            };
            head.checked_add(tail)
        }
        Some(idx) => {
            let left = parse_number_star_(&na[..idx])?;
            let right = if idx + 1 < na.len() {
                parse_number_star_(&na[idx + 1..])?
            } else {
                0
            };
            left.checked_mul(10u128.pow(mp as u32))?.checked_add(right)
        }
    }
}

/// Port of `ichiran/numbers:parse-number` (`numbers.lisp:74`).
///
/// Parse a string of digit / power glyphs (kanji, ASCII, or full-width)
/// into a `u128`. Classifies each character via
/// [`super::helpers::char_number_class_hash`]
/// and delegates the structural arithmetic to
/// [`super::parse_number_star__::parse_number_star_`].
///
/// Returns [`Err`] with a [`NotANumber`] carrying the offending input
/// and a per-character reason when any glyph is unclassifiable, or
/// when the value exceeds `u128` (the Lisp parses such values as
/// bignums; refusing here drops the counter candidate instead).
pub fn parse_number(s: &str) -> Result<u128, NotANumber> {
    let h = char_number_class_hash();
    let mut na = Vec::with_capacity(s.chars().count());
    for c in s.chars() {
        match h.get(&c) {
            Some(&pair) => na.push(pair),
            None => {
                return Err(NotANumber {
                    text: s.to_string(),
                    reason: format!("Invalid character: {c}"),
                });
            }
        }
    }
    parse_number_star_(&na).ok_or_else(|| NotANumber {
        text: s.to_string(),
        reason: "Value exceeds u128 range".to_string(),
    })
}

#[cfg(test)]
mod tests;
