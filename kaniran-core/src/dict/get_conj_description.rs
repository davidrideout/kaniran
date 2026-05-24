//! Port of `ichiran/dict:get-conj-description` (`dict-load.lisp:257`, `csv-hash *conj-description*` accessor).
//!
//! Looks up a conj-id's description in [`conj_description`]; `None` when
//! the id is absent (upstream `gethash` → `nil`).

use super::_star_conj_description_star_::conj_description;

pub fn get_conj_description(key: i32) -> Option<&'static str> {
    conj_description().get(&key).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL fixtures (.103, ichiran/dict::get-conj-description), 2026-05-24.
    /// Present ids resolve to their description; an absent id returns
    /// `None` (upstream nil).
    #[test]
    fn get_conj_description_fixtures() {
        let cases: &[(i32, Option<&str>)] = &[
            (1, Some("Non-past")),
            (2, Some("Past (~ta)")),
            (11, Some("Conditional (~tara)")),
            (50, Some("Adverbial")),
            (999, None),
        ];
        for (key, expected) in cases {
            assert_eq!(get_conj_description(*key), *expected, "key={key}");
        }
    }
}
