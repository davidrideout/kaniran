//! Port of `ichiran/custom:ward-csv` (`dict-custom.lisp:266`).

use std::path::PathBuf;

use super::csv_loader_class::CsvLoader;
use super::custom_source_class::{CustomEntry, CustomSource};
use super::romanize_municipality::romanize_municipality;
use super::ward_struct::Ward;

/// `ward-csv` loader — newtype around `CsvLoader` whose only upstream
/// addition is `(description :initform "wards")`.
///
/// ```text
/// WardCsv(CsvLoader {
///     base: CustomSource {
///         description: "wards".to_string(),
///         entries: vec![],
///     },
///     source_file: PathBuf::from(".../gyoseiku.csv"),
///     csv_options: CsvOptions { separator: ',', skip_first_p: false },
/// })
/// ```
#[derive(Debug, Clone)]
pub struct WardCsv(pub CsvLoader);

impl WardCsv {
    pub fn new(source_file: PathBuf) -> Self {
        let mut inner = CsvLoader::new(source_file);
        // dict-custom.lisp:267 (description :initform "wards")
        inner.base = CustomSource {
            description: "wards".to_string(),
            entries: Vec::new(),
        };
        WardCsv(inner)
    }

    pub fn slurp(&mut self) -> std::io::Result<()> {
        let content = std::fs::read_to_string(&self.0.source_file)?;
        let separator = self.0.csv_options.separator;
        let skip_first_p = self.0.csv_options.skip_first_p;
        let mut entries: Vec<CustomEntry> = Vec::new();
        // dict-custom.lisp:273-274 (with city-text and city-reading and city-romanized)
        let mut city_text: Option<String> = None;
        let mut city_reading: Option<String> = None;
        let mut city_romanized: Option<String> = None;
        for (i, line) in content.lines().enumerate() {
            if i == 0 && skip_first_p {
                continue;
            }
            if line.is_empty() {
                continue;
            }
            // dict-custom.lisp:275 (for (id text reading) in (apply 'cl-csv:read-csv (source-file loader) (csv-options loader)))
            let cols: Vec<&str> = line.split(separator).collect();
            assert!(cols.len() >= 3, "ward-csv row arity (dict-custom.lisp:275)");
            let text = cols[1];
            let reading = cols[2];
            // dict-custom.lisp:276 (if (alexandria:ends-with #\区 text) ...)
            if text.chars().last() == Some('区') {
                let ct = city_text.as_deref().expect("ward-csv: ward row before city row");
                let cr = city_reading
                    .as_deref()
                    .expect("ward-csv: ward row before city row");
                let cr_rom = city_romanized
                    .clone()
                    .expect("ward-csv: ward row before city row");
                // dict-custom.lisp:278 (ward-text (subseq text (length city-text)))
                let ward_text = text.chars().skip(ct.chars().count()).collect::<String>();
                // dict-custom.lisp:279 (ward-reading (subseq reading (length city-reading)))
                let ward_reading = reading.chars().skip(cr.chars().count()).collect::<String>();
                // dict-custom.lisp:280 (definition (format nil "~a, ~a" (romanize-municipality ward-text ward-reading) city-romanized))
                let definition =
                    format!("{}, {}", romanize_municipality(&ward_text, &ward_reading, None), cr_rom);
                // dict-custom.lisp:281 (make-ward :text ward-text :reading ward-reading :definition definition :city city-romanized)
                entries.push(CustomEntry::Ward(Ward {
                    text: ward_text,
                    reading: ward_reading,
                    definition,
                    city: cr_rom,
                }));
            } else {
                // dict-custom.lisp:282-284 (else do (setf city-text text city-reading reading city-romanized (romanize-municipality text reading :include-type nil)))
                city_text = Some(text.to_string());
                city_reading = Some(reading.to_string());
                city_romanized = Some(romanize_municipality(text, reading, Some(false)));
            }
        }
        self.0.base.entries = entries;
        Ok(())
    }
}
