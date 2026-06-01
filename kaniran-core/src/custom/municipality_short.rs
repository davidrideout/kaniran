//! Port of `ichiran/custom:municipality-short` (`dict-custom.lisp:120`).

use super::_star_municipality_types_star_::MUNICIPALITY_TYPES;

pub fn municipality_short(text: &str, reading: &str) -> (String, Option<String>) {
    // dict-custom.lisp:121 (if (alexandria:ends-with #\道 text) (cons text reading) ...)
    if text.chars().last() == Some('道') {
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
    let short_reading: Option<String> = type_readings.iter().find_map(|tpr| {
        if reading.ends_with(tpr) {
            Some(reading[..reading.len() - tpr.len()].to_string())
        } else {
            None
        }
    });
    (short_text, short_reading)
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::municipality-short), 2026-05-31.
    use super::*;

    #[test]
    fn municipality_short_fixtures() {
        let cases: &[(&str, &str, &str, Option<&str>)] = &[
            ("東京都", "とうきょうと", "東京", Some("とうきょう")),
            ("北海道", "ほっかいどう", "北海道", Some("ほっかいどう")),
            ("大阪府", "おおさかふ", "大阪", Some("おおさか")),
            ("沖縄県", "おきなわけん", "沖縄", Some("おきなわ")),
            ("横浜市", "よこはまし", "横浜", Some("よこはま")),
            ("軽井沢町", "かるいざわまち", "軽井沢", Some("かるいざわ")),
            ("葉山町", "はやまちょう", "葉山", Some("はやま")),
            ("葉山町", "はやままち", "葉山", Some("はやま")),
            ("白川村", "しらかわむら", "白川", Some("しらかわ")),
            ("白川村", "しらかわそん", "白川", Some("しらかわ")),
            ("中央区", "ちゅうおうく", "中央", Some("ちゅうおう")),
            ("京都市", "きょうとし", "京都", Some("きょうと")),
            ("横浜市", "ﾖｺﾊﾏｼ", "横浜", Some("ﾖｺﾊﾏ")),
            ("横浜市", "よこはま", "横浜", None),
            ("道", "どう", "道", Some("どう")),
            ("市町村", "しちょうそん", "市町", Some("しちょう")),
        ];
        for (text, reading, expect_text, expect_reading) in cases {
            let (got_text, got_reading) = municipality_short(text, reading);
            assert_eq!(&got_text, expect_text, "text input={text:?} reading={reading:?}");
            assert_eq!(
                got_reading.as_deref(),
                *expect_reading,
                "reading input={text:?} reading={reading:?}",
            );
        }
    }
}
