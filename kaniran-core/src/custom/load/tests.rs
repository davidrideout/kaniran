use super::*;

// --- get_words ---
// REPL fixtures (.103, ichiran/custom::get-words), 2026-05-31.

fn muni(
    text: &str,
    r#type: char,
    reading: &str,
    definition: &str,
    prefecture: Option<&str>,
) -> Municipality {
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
            "東京都",
            '都',
            "とうきょう",
            "Tokyo Metropolis",
            None,
        ))),
        vec!["Tokyo".to_string(), "Metropolis".to_string()],
    );
    assert_eq!(
        get_words(&CustomEntry::Municipality(muni(
            "北海道",
            '道',
            "ほっかいどう",
            "Hokkaido",
            None,
        ))),
        vec!["Hokkaido".to_string()],
    );
    assert_eq!(
        get_words(&CustomEntry::Municipality(muni(
            "横浜市",
            '市',
            "よこはま",
            "Yokohama (city), Kanagawa Prefecture",
            Some("Kanagawa Prefecture"),
        ))),
        vec![
            "Yokohama".to_string(),
            "(city".to_string(),
            "Kanagawa Prefecture".to_string(),
        ],
    );
    assert_eq!(
        get_words(&CustomEntry::Municipality(muni(
            "白川村",
            '村',
            "しらかわ",
            "Shirakawa (village), Gifu Prefecture",
            Some("Gifu Prefecture"),
        ))),
        vec![
            "Shirakawa".to_string(),
            "(village".to_string(),
            "Gifu Prefecture".to_string(),
        ],
    );
    assert_eq!(
        get_words(&CustomEntry::Municipality(muni(
            "神奈川県",
            '県',
            "かながわ",
            "Kanagawa Prefecture",
            None,
        ))),
        vec!["Kanagawa".to_string(), "Prefecture".to_string()],
    );
    assert_eq!(
        get_words(&CustomEntry::Municipality(muni(
            "中央区",
            '区',
            "ちゅうおう",
            "Chuo Ward, Tokyo",
            Some("Tokyo"),
        ))),
        vec!["Chuo".to_string(), "Ward".to_string(), "Tokyo".to_string()],
    );
    assert_eq!(
        get_words(&CustomEntry::Municipality(muni(
            "大阪府",
            '府',
            "おおさか",
            "Osaka Prefecture",
            None,
        ))),
        vec!["Osaka".to_string(), "Prefecture".to_string()],
    );
    assert_eq!(
        get_words(&CustomEntry::Municipality(muni(
            "東京都",
            '都',
            "とうきょう",
            "Tokyo Metropolis",
            None,
        ))),
        vec!["Tokyo".to_string(), "Metropolis".to_string()],
    );
}

#[test]
fn get_words_ward_fixtures() {
    assert_eq!(
        get_words(&CustomEntry::Ward(ward(
            "中央区",
            "ちゅうおうく",
            "Chuo Ward, Tokyo",
            "Tokyo",
        ))),
        vec!["Chuo".to_string(), "Ward".to_string(), "Tokyo".to_string()],
    );
    assert_eq!(
        get_words(&CustomEntry::Ward(ward(
            "北区",
            "きたく",
            "Kita Ward, Sapporo",
            "Sapporo",
        ))),
        vec![
            "Kita".to_string(),
            "Ward".to_string(),
            "Sapporo".to_string()
        ],
    );
}

// --- source_path ---
#[test]
fn source_path_joins_data_sources() {
    let p = source_path("extra.xml");
    assert!(p.is_absolute(), "expected absolute path, got {p:?}");
    assert!(p.ends_with("data/sources/extra.xml"), "got {p:?}");
    let p = source_path("jichitai.csv");
    assert!(p.is_absolute(), "expected absolute path, got {p:?}");
    assert!(p.ends_with("data/sources/jichitai.csv"), "got {p:?}");
    let p = source_path("gyoseiku.csv");
    assert!(p.is_absolute(), "expected absolute path, got {p:?}");
    assert!(p.ends_with("data/sources/gyoseiku.csv"), "got {p:?}");
}

// --- get_custom_data ---
// REPL fixtures (.103, `(ichiran/custom::get-custom-data)`), 2026-05-31.

#[test]
fn get_custom_data_shape() {
    let data = get_custom_data();
    assert_eq!(data.len(), 3);

    // First pair — :extra → xml-loader, description "extra XML data",
    // source-file ends in data/sources/extra.xml.
    assert_eq!(data[0].0, CustomDataKey::Extra);
    match &data[0].1 {
        CustomLoader::Xml(x) => {
            assert_eq!(x.base.description, "extra XML data");
            assert!(
                x.source_file.ends_with("data/sources/extra.xml"),
                "got {:?}",
                x.source_file
            );
        }
        other => panic!("expected Xml loader, got {other:?}"),
    }

    // Second pair — :municipality → municipality-csv, description
    // "municipalities", source-file ends in data/sources/jichitai.csv.
    assert_eq!(data[1].0, CustomDataKey::Municipality);
    match &data[1].1 {
        CustomLoader::Municipality(m) => {
            assert_eq!(m.0.base.description, "municipalities");
            assert!(
                m.0.source_file.ends_with("data/sources/jichitai.csv"),
                "got {:?}",
                m.0.source_file
            );
        }
        other => panic!("expected Municipality loader, got {other:?}"),
    }

    // Third pair — :ward → ward-csv, description "wards",
    // source-file ends in data/sources/gyoseiku.csv.
    assert_eq!(data[2].0, CustomDataKey::Ward);
    match &data[2].1 {
        CustomLoader::Ward(w) => {
            assert_eq!(w.0.base.description, "wards");
            assert!(
                w.0.source_file.ends_with("data/sources/gyoseiku.csv"),
                "got {:?}",
                w.0.source_file
            );
        }
        other => panic!("expected Ward loader, got {other:?}"),
    }
}
