//! Port of `ichiran/custom:municipality-csv` (`dict-custom.lisp:93`).

use std::path::PathBuf;

use super::_star_municipality_types_order_star_::MUNICIPALITY_TYPES_ORDER;
use super::csv_loader_class::CsvLoader;
use super::custom_source_class::{CustomEntry, CustomSource};
use super::municipality_short::municipality_short;
use super::municipality_struct::Municipality;
use super::romanize_municipality::romanize_municipality;
use crate::characters::as_hiragana::as_hiragana;
use crate::characters::normalize::normalize;
use crate::characters::to_normal_char::NormalizationContext;

/// `municipality-csv` loader — newtype around `CsvLoader` whose only
/// upstream addition is `(description :initform "municipalities")`.
///
/// ```text
/// MunicipalityCsv(CsvLoader {
///     base: CustomSource {
///         description: "municipalities".to_string(),
///         entries: vec![],
///     },
///     source_file: PathBuf::from(".../jichitai.csv"),
///     csv_options: CsvOptions { separator: ',', skip_first_p: false },
/// })
/// ```
#[derive(Debug, Clone)]
pub struct MunicipalityCsv(pub CsvLoader);

impl MunicipalityCsv {
    pub fn new(source_file: PathBuf) -> Self {
        let mut inner = CsvLoader::new(source_file);
        // dict-custom.lisp:94 (description :initform "municipalities")
        inner.base = CustomSource {
            description: "municipalities".to_string(),
            entries: Vec::new(),
        };
        MunicipalityCsv(inner)
    }

    pub fn slurp(&mut self) -> std::io::Result<()> {
        // dict-custom.lisp:88-91 (defmethod slurp ((loader csv-loader)) (setf (entries loader) (loop for row in (apply 'cl-csv:read-csv (source-file loader) (csv-options loader)) nconc (process-entry loader row))))
        let content = std::fs::read_to_string(&self.0.source_file)?;
        let separator = self.0.csv_options.separator;
        let skip_first_p = self.0.csv_options.skip_first_p;
        let mut entries: Vec<CustomEntry> = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if i == 0 && skip_first_p {
                continue;
            }
            if line.is_empty() {
                continue;
            }
            let row: Vec<String> = line.split(separator).map(str::to_string).collect();
            entries.extend(
                self.process_entry(&row)
                    .into_iter()
                    .map(CustomEntry::Municipality),
            );
        }
        // dict-custom.lisp:168-171 (defmethod slurp :after ((loader municipality-csv)) (setf (entries loader) (stable-sort (entries loader) '< :key (lambda (e) (position (municipality-type e) *municipality-types-order*)))))
        entries.sort_by_key(|e| match e {
            CustomEntry::Municipality(m) => MUNICIPALITY_TYPES_ORDER
                .chars()
                .position(|c| c == m.r#type)
                .expect("municipality type must be in *municipality-types-order*"),
            other => panic!("municipality-csv slurp produced non-municipality entry: {other:?}"),
        });
        self.0.base.entries = entries;
        Ok(())
    }

    pub fn process_entry(&self, row: &[String]) -> Vec<Municipality> {
        // dict-custom.lisp:143 (destructuring-bind (id pref muni rpref rmuni) row)
        assert_eq!(row.len(), 5, "municipality-csv row arity (dict-custom.lisp:143)");
        let pref = &row[1];
        let muni = &row[2];
        let rpref = &row[3];
        let rmuni = &row[4];
        // dict-custom.lisp:145 (prefecture-p (alexandria:emptyp muni))
        let prefecture_p = muni.is_empty();
        // dict-custom.lisp:146 (text (if prefecture-p pref muni))
        let text: &str = if prefecture_p { pref } else { muni };
        // dict-custom.lisp:147 (type (char text (1- (length text))))
        let r#type = text.chars().last().expect("process-entry: empty text");
        // dict-custom.lisp:148 (reading (if prefecture-p rpref rmuni))
        let reading: &str = if prefecture_p { rpref } else { rmuni };
        // dict-custom.lisp:149 (short (municipality-short text reading))
        let short = municipality_short(text, reading);
        // dict-custom.lisp:150 (prefecture (unless prefecture-p (romanize-municipality pref rpref)))
        let prefecture = if prefecture_p {
            None
        } else {
            Some(romanize_municipality(pref, rpref, None))
        };
        // dict-custom.lisp:151-153 (definition (format nil "~a~@[, ~a~]" (romanize-municipality text reading) prefecture))
        let romanized_self = romanize_municipality(text, reading, None);
        let definition = match &prefecture {
            Some(p) => format!("{}, {}", romanized_self, p),
            None => romanized_self,
        };
        // dict-custom.lisp:154-157 (muni (make-municipality :text text :type type :reading (as-hiragana (normalize reading)) :definition definition :prefecture prefecture))
        let muni_long = Municipality {
            text: text.to_string(),
            reading: as_hiragana(&normalize(reading, NormalizationContext::Default)),
            definition: definition.clone(),
            r#type,
            prefecture: prefecture.clone(),
        };
        // dict-custom.lisp:158-163 (muni-short (unless (find type "道") (make-municipality :text (car short) :type type :reading (as-hiragana (normalize (cdr short))) :definition definition :prefecture prefecture)))
        if r#type == '道' {
            return vec![muni_long];
        }
        let short_reading = short.1.expect(
            "process-entry: short-reading nil for non-道 type — CSV reading must end in a known suffix",
        );
        let muni_short = Municipality {
            text: short.0,
            reading: as_hiragana(&normalize(&short_reading, NormalizationContext::Default)),
            definition,
            r#type,
            prefecture,
        };
        // dict-custom.lisp:164-166 (if muni-short (list muni muni-short) (list muni))
        vec![muni_long, muni_short]
    }
}
