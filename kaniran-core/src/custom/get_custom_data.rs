//! Port of `ichiran/custom:get-custom-data` (`dict-custom.lisp:318`).
//!
//! Returns the three built-in loaders, each tagged with the keyword the
//! Lisp plist used (`:extra`, `:municipality`, `:ward`). The upstream
//! mapcar over a flat plist collapses here to a list of pairs per
//! CONVENTIONS §4.3 — every consumer already pairs the alternating
//! keyword/loader cells via `(loop ... by #'cddr)`.

use super::custom_source_class::CustomLoader;
use super::municipality_csv_class::MunicipalityCsv;
use super::source_path::source_path;
use super::ward_csv_class::WardCsv;
use super::xml_loader_class::XmlLoader;

/// Tag identifying one of the built-in custom-data loaders.
///
/// ```text
/// CustomDataKey::Extra         // :extra
/// CustomDataKey::Municipality  // :municipality
/// CustomDataKey::Ward          // :ward
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomDataKey {
    Extra,
    Municipality,
    Ward,
}

pub fn get_custom_data() -> Vec<(CustomDataKey, CustomLoader)> {
    // dict-custom.lisp:321-324 (`( :extra (xml-loader :source-file ,(source-path "extra.xml"))
    //                             :municipality (municipality-csv :source-file ,(source-path "jichitai.csv"))
    //                             :ward (ward-csv :source-file ,(source-path "gyoseiku.csv")) ))
    vec![
        (
            CustomDataKey::Extra,
            CustomLoader::Xml(XmlLoader::new(source_path("extra.xml"))),
        ),
        (
            CustomDataKey::Municipality,
            CustomLoader::Municipality(MunicipalityCsv::new(source_path("jichitai.csv"))),
        ),
        (
            CustomDataKey::Ward,
            CustomLoader::Ward(WardCsv::new(source_path("gyoseiku.csv"))),
        ),
    ]
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, `(ichiran/custom::get-custom-data)`), 2026-05-31.
    use super::*;

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
}
