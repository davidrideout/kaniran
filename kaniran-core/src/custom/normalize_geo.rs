//! Port of `ichiran/custom:normalize-geo` (`dict-custom.lisp:173`).

use crate::characters::simplify_ngrams::simplify_ngrams;

pub fn normalize_geo(word: &str) -> String {
    // dict-custom.lisp:174 (simplify-ngrams (string-downcase word) '("ū" "u" "ō" "o"))
    simplify_ngrams(&word.to_lowercase(), &[("ū", "u"), ("ō", "o")])
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::normalize-geo), 2026-05-31.
    use super::*;

    #[test]
    fn normalize_geo_fixtures() {
        let cases: &[(&str, &str)] = &[
            ("Tokyo", "tokyo"),
            ("Tōkyō", "tokyo"),
            ("Ōsaka", "osaka"),
            ("Hokkaido", "hokkaido"),
            ("HŪNŌ", "huno"),
            ("", ""),
            ("TŌKYŌ-DOU", "tokyo-dou"),
            ("Kyōto", "kyoto"),
            ("Shin'Ōsaka", "shin'osaka"),
        ];
        for (input, expected) in cases {
            assert_eq!(&normalize_geo(input), expected, "input={input}");
        }
    }
}
