use super::*;
use crate::custom::types::{MunicipalityCsv, WardCsv};
use std::path::PathBuf;
use std::sync::Arc;

// --- process_entry ---

fn row(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}

#[test]
#[should_panic(expected = "short-reading nil")]
fn process_entry_municipality_panics_on_non_dou_with_no_suffix() {
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let row = row(&["999000", "神奈川県", "横浜市", "ｶﾅｶﾞﾜｹﾝ", "よこはま"]);
    let _ = process_entry(&loader, &row);
}

#[test]
fn process_entry_municipality_prefecture_fu_two_items() {
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let row = row(&["270008", "大阪府", "", "ｵｵｻｶﾌ", ""]);
    let out = process_entry(&loader, &row);
    assert_eq!(out.len(), 2);
    match &out[0] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "大阪府");
            assert_eq!(m.r#type, '府');
            assert_eq!(m.reading, "おおさかふ");
            assert_eq!(m.definition, "Osaka Prefecture");
            assert_eq!(m.prefecture, None);
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
    match &out[1] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "大阪");
            assert_eq!(m.r#type, '府');
            assert_eq!(m.reading, "おおさか");
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
}

#[test]
fn process_entry_municipality_village_two_items() {
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let row = row(&["215007", "岐阜県", "白川村", "ｷﾞﾌｹﾝ", "ｼﾗｶﾜﾑﾗ"]);
    let out = process_entry(&loader, &row);
    assert_eq!(out.len(), 2);
    match &out[0] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "白川村");
            assert_eq!(m.r#type, '村');
            assert_eq!(m.reading, "しらかわむら");
            assert_eq!(m.definition, "Shirakawa (village), Gifu Prefecture");
            assert_eq!(m.prefecture.as_deref(), Some("Gifu Prefecture"));
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
    match &out[1] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "白川");
            assert_eq!(m.r#type, '村');
            assert_eq!(m.reading, "しらかわ");
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
}

#[test]
fn process_entry_municipality_prefecture_to_two_items() {
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let row = row(&["130001", "東京都", "", "ﾄｳｷｮｳﾄ", ""]);
    let out = process_entry(&loader, &row);
    assert_eq!(out.len(), 2);
    match &out[0] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "東京都");
            assert_eq!(m.r#type, '都');
            assert_eq!(m.reading, "とうきょうと");
            assert_eq!(m.definition, "Tokyo Metropolis");
            assert_eq!(m.prefecture, None);
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
    match &out[1] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "東京");
            assert_eq!(m.r#type, '都');
            assert_eq!(m.reading, "とうきょう");
            assert_eq!(m.definition, "Tokyo Metropolis");
            assert_eq!(m.prefecture, None);
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
}

#[test]
fn process_entry_municipality_prefecture_dou_one_item() {
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let row = row(&["010006", "北海道", "", "ﾎｯｶｲﾄﾞｳ", ""]);
    let out = process_entry(&loader, &row);
    assert_eq!(out.len(), 1);
    match &out[0] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "北海道");
            assert_eq!(m.r#type, '道');
            assert_eq!(m.reading, "ほっかいどう");
            assert_eq!(m.definition, "Hokkaido");
            assert_eq!(m.prefecture, None);
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
}

#[test]
fn process_entry_municipality_city_two_items() {
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let row = row(&["011002", "北海道", "札幌市", "ﾎｯｶｲﾄﾞｳ", "ｻｯﾎﾟﾛｼ"]);
    let out = process_entry(&loader, &row);
    assert_eq!(out.len(), 2);
    match &out[0] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "札幌市");
            assert_eq!(m.r#type, '市');
            assert_eq!(m.reading, "さっぽろし");
            assert_eq!(m.definition, "Sapporo (city), Hokkaido");
            assert_eq!(m.prefecture.as_deref(), Some("Hokkaido"));
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
    match &out[1] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "札幌");
            assert_eq!(m.r#type, '市');
            assert_eq!(m.reading, "さっぽろ");
            assert_eq!(m.definition, "Sapporo (city), Hokkaido");
            assert_eq!(m.prefecture.as_deref(), Some("Hokkaido"));
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
}

#[test]
fn process_entry_municipality_town_two_items() {
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let row = row(&["142042", "神奈川県", "葉山町", "ｶﾅｶﾞﾜｹﾝ", "ﾊﾔﾏﾏﾁ"]);
    let out = process_entry(&loader, &row);
    assert_eq!(out.len(), 2);
    match &out[0] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "葉山町");
            assert_eq!(m.r#type, '町');
            assert_eq!(m.reading, "はやままち");
            assert_eq!(m.definition, "Hayama (town), Kanagawa Prefecture");
            assert_eq!(m.prefecture.as_deref(), Some("Kanagawa Prefecture"));
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
    match &out[1] {
        CustomEntry::Municipality(m) => {
            assert_eq!(m.text, "葉山");
            assert_eq!(m.r#type, '町');
            assert_eq!(m.reading, "はやま");
        }
        other => panic!("expected Municipality, got {other:?}"),
    }
}

// --- test_entry ---
// Needs a live Postgres database.

async fn ctx_from_env() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        .await
        .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
}

/// An entry already in the dictionary is skipped.
#[tokio::test]
async fn test_entry_municipality_skip_path() {
    let ctx = ctx_from_env().await;
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let entry = CustomEntry::Municipality(Municipality {
        text: "東京".to_string(),
        reading: "とうきょう".to_string(),
        definition: "Tokyo Metropolis".to_string(),
        r#type: '都',
        prefecture: None,
    });
    let got = test_entry(&ctx, &loader, &entry).await.unwrap();
    assert_eq!(got, TestEntryResult::Skip);
}

/// An entry with no matching dictionary candidate is inserted.
#[tokio::test]
async fn test_entry_municipality_insert_path() {
    let ctx = ctx_from_env().await;
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let entry = CustomEntry::Municipality(Municipality {
        text: "ZZZ".to_string(),
        reading: "うあぱ".to_string(),
        definition: "ZZZ (city), Foo".to_string(),
        r#type: '市',
        prefecture: Some("Foo".to_string()),
    });
    let got = test_entry(&ctx, &loader, &entry).await.unwrap();
    assert_eq!(got, TestEntryResult::Insert);
}

/// An entry matching an existing sequence updates that sequence.
#[tokio::test]
async fn test_entry_municipality_update_path() {
    let ctx = ctx_from_env().await;
    let loader = CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
    let entry = CustomEntry::Municipality(Municipality {
        text: "漢字".to_string(),
        reading: "かんじ".to_string(),
        definition: "FAKE definition with xxx".to_string(),
        r#type: '市',
        prefecture: Some("FakePref Prefecture".to_string()),
    });
    let got = test_entry(&ctx, &loader, &entry).await.unwrap();
    assert_eq!(got, TestEntryResult::Update(1213170));
}

/// A ward entry already in the dictionary is skipped.
#[tokio::test]
async fn test_entry_ward_skip_path() {
    let ctx = ctx_from_env().await;
    let loader = CustomLoader::Ward(WardCsv::new(PathBuf::from("/tmp/x.csv")));
    let entry = CustomEntry::Ward(Ward {
        text: "中央区".to_string(),
        reading: "ちゅうおうく".to_string(),
        definition: "Chuo Ward, Sapporo".to_string(),
        city: "Sapporo".to_string(),
    });
    let got = test_entry(&ctx, &loader, &entry).await.unwrap();
    assert_eq!(got, TestEntryResult::Skip);
}

#[test]
fn city_update_gloss_regex_shape() {
    let words = vec![
        "Yokohama".to_string(),
        "(city".to_string(),
        "Kanagawa Prefecture".to_string(),
    ];
    let entry = Municipality {
        text: "横浜".to_string(),
        reading: "よこはま".to_string(),
        definition: "Yokohama (city), Kanagawa Prefecture".to_string(),
        r#type: '市',
        prefecture: Some("Kanagawa Prefecture".to_string()),
    };
    let rg = build_city_update_gloss(&words, &entry);
    assert!(rg.is_match("Yokohama (city)").unwrap());
    assert!(rg.is_match("YOKOHAMA (CITY)").unwrap());
    assert!(rg.is_match("Yokohama (city in Kanagawa)").unwrap());
    assert!(!rg.is_match("Yokohama (city), Kanagawa Prefecture").unwrap());
    assert!(!rg.is_match("Yokohama (city in Tokyo)").unwrap());
}

#[test]
fn pref_update_gloss_regex_shape() {
    let words = vec!["Kanagawa".to_string(), "Prefecture".to_string()];
    let rg = build_pref_update_gloss(&words);
    assert!(rg.is_match("Kanagawa (prefecture)").unwrap());
    assert!(rg.is_match("Kanagawa (city, prefecture)").unwrap());
    assert!(rg.is_match("KANAGAWA (PREFECTURE)").unwrap());
    assert!(!rg.is_match("Tokyo (prefecture)").unwrap());
    assert!(!rg.is_match("Kanagawa Prefecture").unwrap());
}
