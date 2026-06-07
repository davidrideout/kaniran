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
pub fn number_to_kanji(n: u64, digits: &str, powers: &str, one_sen: bool) -> String {
    let digit_chars: Vec<char> = digits.chars().collect();
    let power_chars: Vec<char> = powers.chars().collect();
    if n == 0 {
        return digit_chars[0].to_string();
    }
    let mut mp: u64 = 1;
    let mut mc: char = power_chars[0];
    let mut p: u64 = 1;
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
    let head_threshold: u64 = if one_sen { 100 } else { 1000 };
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
        None => na.iter().fold(0u64, |a, &(_class, v)| a * 10 + v as u64),
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

/// Port of `ichiran/numbers:parse-number` (`numbers.lisp:74`).
///
/// Parse a string of digit / power glyphs (kanji, ASCII, or full-width)
/// into a `u64`. Classifies each character via
/// [`super::helpers::char_number_class_hash`]
/// and delegates the structural arithmetic to
/// [`super::parse_number_star__::parse_number_star_`].
///
/// Returns [`Err`] with a [`NotANumber`] carrying the offending input
/// and a per-character reason when any glyph is unclassifiable.
pub fn parse_number(s: &str) -> Result<u64, NotANumber> {
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
    Ok(parse_number_star_(&na))
}

#[cfg(test)]
mod tests;
