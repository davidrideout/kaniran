//! Port of `ichiran/custom:romanize-municipality` (`dict-custom.lisp:132`).
//!
//! `include_type` is `Option<bool>` so the upstream `&key (include-type t)`
//! default lives at the boundary, not at every call site.

use super::_star_municipality_types_description_star_::MUNICIPALITY_TYPES_DESCRIPTION;
use super::municipality_short::municipality_short;
use crate::core::_star_hepburn_simple_star_::hepburn_simple;
use crate::core::generic_romanization_class::RomanizationMethod;
use crate::core::romanize_word_geo::romanize_word_geo;

pub fn romanize_municipality(text: &str, reading: &str, include_type: Option<bool>) -> String {
    // dict-custom.lisp:132 — `&key (include-type t)`. `None` here means
    // "caller didn't supply :INCLUDE-TYPE", which Lisp resolves to `t`.
    let include_type = include_type.unwrap_or(true);
    // dict-custom.lisp:133 (short-reading (cdr (municipality-short text reading)))
    let short_reading = municipality_short(text, reading).1;
    // dict-custom.lisp:134 (type (char text (1- (length text))))
    let r#type = text.chars().last().expect("romanize-municipality: empty text");
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

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::romanize-municipality), 2026-05-31.
    use super::*;

    #[test]
    fn romanize_municipality_fixtures() {
        let cases: &[(&str, &str, Option<bool>, &str)] = &[
            ("東京都", "とうきょうと", None, "Tokyo Metropolis"),
            ("東京都", "とうきょうと", Some(true), "Tokyo Metropolis"),
            ("東京都", "とうきょうと", Some(false), "Tokyo"),
            ("北海道", "ほっかいどう", None, "Hokkaido"),
            ("大阪府", "おおさかふ", None, "Osaka Prefecture"),
            ("沖縄県", "おきなわけん", None, "Okinawa Prefecture"),
            ("横浜市", "よこはまし", None, "Yokohama (city)"),
            ("鎌倉市", "かまくらし", None, "Kamakura (city)"),
            ("軽井沢町", "かるいざわまち", None, "Karuizawa (town)"),
            ("白川村", "しらかわむら", None, "Shirakawa (village)"),
            ("中央区", "ちゅうおうく", None, "Chuo Ward"),
            ("京都市", "きょうとし", Some(false), "Kyoto"),
            ("横浜市", "よこはま", None, " (city)"),
        ];
        for (text, reading, include_type, expected) in cases {
            let got = romanize_municipality(text, reading, *include_type);
            assert_eq!(
                &got, expected,
                "input text={text:?} reading={reading:?} include_type={include_type:?}",
            );
        }
    }
}
