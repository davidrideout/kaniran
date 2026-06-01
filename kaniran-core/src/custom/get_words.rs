//! Port of `ichiran/custom:get-words` (gf — `dict-custom.lisp:176`).

use super::_star_municipality_types_description_star_::MUNICIPALITY_TYPES_DESCRIPTION;
use super::custom_source_class::CustomEntry;
use super::municipality_struct::Municipality;
use super::ward_struct::Ward;

pub fn get_words(entry: &CustomEntry) -> Vec<String> {
    match entry {
        CustomEntry::Municipality(m) => get_words_municipality(m),
        CustomEntry::Ward(w) => get_words_ward(w),
        CustomEntry::Xml(_) => {
            panic!("get-words: no method for xml-entry (dict-custom.lisp:176)")
        }
    }
}

fn get_words_municipality(entry: &Municipality) -> Vec<String> {
    // dict-custom.lisp:178 (typeword (cdr (assoc (municipality-type entry) *municipality-types-description*)))
    let mut typeword: Option<String> = MUNICIPALITY_TYPES_DESCRIPTION
        .iter()
        .find_map(|(t, d)| if *t == entry.r#type { d.map(|s| s.to_string()) } else { None });
    // dict-custom.lisp:179 (name (car (split-sequence #\Space (municipality-definition entry))))
    let name = entry
        .definition
        .split(' ')
        .next()
        .expect("split always yields at least one element")
        .to_string();
    // dict-custom.lisp:180-181 (when (and typeword (alexandria:ends-with #\) typeword)) (setf typeword (subseq typeword 0 (1- (length typeword)))))
    if let Some(tw) = &typeword {
        if tw.ends_with(')') {
            let trimmed = tw[..tw.len() - ')'.len_utf8()].to_string();
            typeword = Some(trimmed);
        }
    }
    // dict-custom.lisp:183 (remove-if 'null (list name typeword (municipality-prefecture entry)))
    let mut out = vec![name];
    if let Some(tw) = typeword {
        out.push(tw);
    }
    if let Some(p) = &entry.prefecture {
        out.push(p.clone());
    }
    out
}

fn get_words_ward(entry: &Ward) -> Vec<String> {
    // dict-custom.lisp:287 (name (car (split-sequence #\Space (ward-definition entry))))
    let name = entry
        .definition
        .split(' ')
        .next()
        .expect("split always yields at least one element")
        .to_string();
    // dict-custom.lisp:288 (list name "Ward" (ward-city entry))
    vec![name, "Ward".to_string(), entry.city.clone()]
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::get-words), 2026-05-31.
    use super::*;

    fn muni(text: &str, r#type: char, reading: &str, definition: &str, prefecture: Option<&str>) -> Municipality {
        Municipality {
            text: text.to_string(),
            reading: reading.to_string(),
            definition: definition.to_string(),
            r#type,
            prefecture: prefecture.map(str::to_string),
        }
    }

    fn ward(text: &str, reading: &str, definition: &str, city: &str) -> Ward {
        Ward {
            text: text.to_string(),
            reading: reading.to_string(),
            definition: definition.to_string(),
            city: city.to_string(),
        }
    }

    #[test]
    fn get_words_municipality_fixtures() {
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "東京都", '都', "とうきょう", "Tokyo Metropolis", None,
            ))),
            vec!["Tokyo".to_string(), "Metropolis".to_string()],
        );
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "北海道", '道', "ほっかいどう", "Hokkaido", None,
            ))),
            vec!["Hokkaido".to_string()],
        );
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "横浜市", '市', "よこはま", "Yokohama (city), Kanagawa Prefecture", Some("Kanagawa Prefecture"),
            ))),
            vec![
                "Yokohama".to_string(),
                "(city".to_string(),
                "Kanagawa Prefecture".to_string(),
            ],
        );
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "白川村", '村', "しらかわ", "Shirakawa (village), Gifu Prefecture", Some("Gifu Prefecture"),
            ))),
            vec![
                "Shirakawa".to_string(),
                "(village".to_string(),
                "Gifu Prefecture".to_string(),
            ],
        );
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "神奈川県", '県', "かながわ", "Kanagawa Prefecture", None,
            ))),
            vec!["Kanagawa".to_string(), "Prefecture".to_string()],
        );
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "中央区", '区', "ちゅうおう", "Chuo Ward, Tokyo", Some("Tokyo"),
            ))),
            vec!["Chuo".to_string(), "Ward".to_string(), "Tokyo".to_string()],
        );
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "大阪府", '府', "おおさか", "Osaka Prefecture", None,
            ))),
            vec!["Osaka".to_string(), "Prefecture".to_string()],
        );
        assert_eq!(
            get_words(&CustomEntry::Municipality(muni(
                "東京都", '都', "とうきょう", "Tokyo Metropolis", None,
            ))),
            vec!["Tokyo".to_string(), "Metropolis".to_string()],
        );
    }

    #[test]
    fn get_words_ward_fixtures() {
        assert_eq!(
            get_words(&CustomEntry::Ward(ward(
                "中央区", "ちゅうおうく", "Chuo Ward, Tokyo", "Tokyo",
            ))),
            vec!["Chuo".to_string(), "Ward".to_string(), "Tokyo".to_string()],
        );
        assert_eq!(
            get_words(&CustomEntry::Ward(ward(
                "北区", "きたく", "Kita Ward, Sapporo", "Sapporo",
            ))),
            vec!["Kita".to_string(), "Ward".to_string(), "Sapporo".to_string()],
        );
    }
}
