use super::constants::{MUNICIPALITY_TYPES, MUNICIPALITY_TYPES_DESCRIPTION};
use kaniran_core::characters::kani_ngram_scanner::KaniNgramScanner;
use kaniran_core::core::methods::hepburn_simple;
use kaniran_core::core::methods::RomanizationMethod;
use kaniran_core::core::romanize::romanize_word_geo;
use std::sync::LazyLock;

/// Port of `ichiran/custom:municipality-short` (`dict-custom.lisp:120`).
pub fn municipality_short(text: &str, reading: &str) -> (String, Option<String>) {
    // dict-custom.lisp:121 (if (alexandria:ends-with #\道 text) (cons text reading) ...)
    if text.ends_with('道') {
        return (text.to_string(), Some(reading.to_string()));
    }
    // dict-custom.lisp:123 (type (char text (1- (length text))))
    let r#type = text.chars().last().expect("municipality-short: empty text");
    let type_len = r#type.len_utf8();
    // dict-custom.lisp:124 (short-text (subseq text 0 (1- (length text))))
    let short_text = text[..text.len() - type_len].to_string();
    // dict-custom.lisp:125 (type-readings (cdr (assoc type *municipality-types*)))
    let type_readings: &[&str] = MUNICIPALITY_TYPES
        .iter()
        .find_map(|(t, r)| if *t == r#type { Some(*r) } else { None })
        .unwrap_or(&[]);
    // dict-custom.lisp:126-129 (loop for tpr in type-readings thereis (and (alexandria:ends-with-subseq tpr reading) (subseq reading 0 (- (length reading) (length tpr)))))
    let short_reading: Option<String> = type_readings
        .iter()
        .find_map(|tpr| reading.strip_suffix(tpr).map(str::to_string));
    (short_text, short_reading)
}

/// Port of `ichiran/custom:romanize-municipality` (`dict-custom.lisp:132`).
///
/// Romanizes a municipality's reading, optionally appending its
/// type description (e.g. "City", "Ward").
pub fn romanize_municipality(text: &str, reading: &str, include_type: Option<bool>) -> String {
    // dict-custom.lisp:132 — `&key (include-type t)`. `None` here means
    // "caller didn't supply :INCLUDE-TYPE", which Lisp resolves to `t`.
    let include_type = include_type.unwrap_or(true);
    // dict-custom.lisp:133 (short-reading (cdr (municipality-short text reading)))
    let short_reading = municipality_short(text, reading).1;
    // dict-custom.lisp:134 (type (char text (1- (length text))))
    let r#type = text
        .chars()
        .last()
        .expect("romanize-municipality: empty text");
    // dict-custom.lisp:135-138 (format nil "~a~@[ ~a~]" (ichiran:romanize-word-geo short-reading) (and include-type (cdr (assoc type *municipality-types-description*))))
    let romanized = romanize_word_geo(
        short_reading.as_deref().unwrap_or(""),
        RomanizationMethod::SimplifiedHepburn(hepburn_simple()),
    );
    let type_desc: Option<&str> = if include_type {
        MUNICIPALITY_TYPES_DESCRIPTION
            .iter()
            .find_map(|(t, d)| if *t == r#type { *d } else { None })
    } else {
        None
    };
    match type_desc {
        Some(desc) => format!("{} {}", romanized, desc),
        None => romanized,
    }
}

/// Port of `ichiran/custom:normalize-geo` (`dict-custom.lisp:173`).
pub fn normalize_geo(word: &str) -> String {
    // dict-custom.lisp:174 (simplify-ngrams (string-downcase word) '("ū" "u" "ō" "o"))
    static SCANNER: LazyLock<KaniNgramScanner> =
        LazyLock::new(|| KaniNgramScanner::new(&[("ū", "u"), ("ō", "o")]));
    SCANNER.simplify(&word.to_lowercase())
}

#[cfg(test)]
mod tests;
