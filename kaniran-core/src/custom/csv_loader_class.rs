//! Port of `ichiran/custom:csv-loader` (`dict-custom.lisp:82`).

use std::path::PathBuf;

use super::custom_source_class::CustomSource;

/// CSV-reader options matching the upstream
/// `(:separator #\, :skip-first-p nil)` default.
///
/// ```text
/// CsvOptions { separator: ',', skip_first_p: false }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CsvOptions {
    pub separator: char,
    pub skip_first_p: bool,
}

impl Default for CsvOptions {
    fn default() -> Self {
        // dict-custom.lisp:85 (csv-options :initform '(:separator #\, :skip-first-p nil))
        CsvOptions {
            separator: ',',
            skip_first_p: false,
        }
    }
}

/// `csv-loader` slots — file path, csv options, plus the inherited
/// base. Abstract parent; `municipality-csv` and `ward-csv` are the
/// concrete subclasses.
///
/// ```text
/// CsvLoader {
///     base: CustomSource {
///         description: "csv".to_string(),
///         entries: vec![],
///     },
///     source_file: PathBuf::from("kaniran-core/data/sources/jichitai.csv"),
///     csv_options: CsvOptions { separator: ',', skip_first_p: false },
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CsvLoader {
    pub base: CustomSource,
    pub source_file: PathBuf,
    pub csv_options: CsvOptions,
}

impl CsvLoader {
    pub fn new(source_file: PathBuf) -> Self {
        CsvLoader {
            // dict-custom.lisp:83 (description :initform "csv")
            base: CustomSource {
                description: "csv".to_string(),
                entries: Vec::new(),
            },
            source_file,
            csv_options: CsvOptions::default(),
        }
    }
}
