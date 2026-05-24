//! Port of `ichiran:r-special` (gf — `romanize.lisp:210-215`).
//!
//! Romanizes the standalone glyphs that carry no mora class: a lone small
//! tsu and the long-vowel bar. The `or` method combination currently has
//! only the `(method word)` default, so the result does not depend on the
//! method; `nil` (no special case) maps to `None`.

use super::generic_romanization_class::RomanizationMethod;

pub fn r_special(method: RomanizationMethod<'_>, word: &str) -> Option<String> {
    let _ = method;
    // romanize.lisp:213-215 (r-special or — default method)
    match word {
        "っ" => Some("!".to_string()),
        "ー" => Some("~".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::traditional_hepburn_class::TraditionalHepburn;

    #[test]
    fn r_special_fixtures() {
        // REPL fixtures (.103, ichiran::r-special), 2026-05-24 — result is
        // method-independent (only the default `or` method exists).
        let traditional = TraditionalHepburn::new();
        let method = RomanizationMethod::TraditionalHepburn(&traditional);
        assert_eq!(r_special(method, "っ"), Some("!".to_string()));
        assert_eq!(r_special(method, "ー"), Some("~".to_string()));
        assert_eq!(r_special(method, "あ"), None);
    }
}
