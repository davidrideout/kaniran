//! Port of `ichiran/custom:process-entry` (gf — `dict-custom.lisp:19`).

use super::custom_source_class::{CustomEntry, CustomLoader};

pub fn process_entry(loader: &CustomLoader, row: &[String]) -> Vec<CustomEntry> {
    match loader {
        // dict-custom.lisp:142 (defmethod process-entry ((loader municipality-csv) row) ...)
        CustomLoader::Municipality(m) => m
            .process_entry(row)
            .into_iter()
            .map(CustomEntry::Municipality)
            .collect(),
        // dict-custom.lisp:21 (:method (source entry) (list entry))
        _ => panic!("process-entry: default method received non-CustomEntry row"),
    }
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::process-entry on a stub
    //! municipality-csv loader), 2026-05-31.
    use super::*;
    use crate::custom::municipality_csv_class::MunicipalityCsv;
    use std::path::PathBuf;

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
}
