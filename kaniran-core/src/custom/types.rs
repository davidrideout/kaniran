use super::constants::MUNICIPALITY_TYPES_ORDER;
use super::geo::{municipality_short, romanize_municipality};
use crate::characters::as_hiragana::as_hiragana;
use crate::characters::normalize::normalize;
use crate::characters::to_normal_char::NormalizationContext;
use crate::conn::kani_context::KaniranContext;
use crate::dict::load_entry::{load_entry, LoadEntryIfExists, LoadEntrySeq};
use crate::dict::node_text::node_text;
use roxmltree::Document;
use std::path::PathBuf;

/// Port of `ichiran/custom:custom-source` (`dict-custom.lisp:54`).
/// Base class slots — `description` plus the polymorphic `entries`
/// vector that every subclass fills during [`super::load::slurp`].
///
/// ```text
/// CustomSource {
///     description: "extra XML data".to_string(),
///     entries: vec![],
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CustomSource {
    pub description: String,
    pub entries: Vec<CustomEntry>,
}

/// Family dispatcher for the loader subclasses. `csv-loader` is
/// abstract and never instantiated, so it has no variant.
#[derive(Debug, Clone)]
pub enum CustomLoader {
    Xml(XmlLoader),
    Municipality(MunicipalityCsv),
    Ward(WardCsv),
}

impl CustomLoader {
    pub fn base(&self) -> &CustomSource {
        match self {
            CustomLoader::Xml(x) => &x.base,
            CustomLoader::Municipality(m) => &m.0.base,
            CustomLoader::Ward(w) => &w.0.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut CustomSource {
        match self {
            CustomLoader::Xml(x) => &mut x.base,
            CustomLoader::Municipality(m) => &mut m.0.base,
            CustomLoader::Ward(w) => &mut w.0.base,
        }
    }
}

/// Rust-only dispatcher for the entry-typed generic functions
/// (`as-xml`, `get-words`, `test-entry` on entries). The three
/// upstream entry types are `defstruct`s with no shared base.
#[derive(Debug, Clone)]
pub enum CustomEntry {
    Xml(XmlEntry),
    Municipality(Municipality),
    Ward(Ward),
}

/// Port of `ichiran/custom:xml-loader` (`dict-custom.lisp:59`).
/// `xml-loader` slots — the file path plus the inherited base.
///
/// ```text
/// XmlLoader {
///     base: CustomSource {
///         description: "extra XML data".to_string(),
///         entries: vec![],
///     },
///     source_file: PathBuf::from("kaniran-core/data/sources/extra.xml"),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct XmlLoader {
    pub base: CustomSource,
    pub source_file: PathBuf,
}

impl XmlLoader {
    pub fn new(source_file: PathBuf) -> Self {
        XmlLoader {
            // dict-custom.lisp:60 (description :initform "extra XML data")
            base: CustomSource {
                description: "extra XML data".to_string(),
                entries: Vec::new(),
            },
            source_file,
        }
    }

    pub fn slurp(&mut self) -> std::io::Result<()> {
        // dict-custom.lisp:66 (content (uiop:read-file-string (source-file loader)))
        let content = std::fs::read_to_string(&self.source_file)?;
        // dict-custom.lisp:67 (parsed (cxml:parse content (cxml-dom:make-dom-builder)))
        let parsed = Document::parse(&content).expect("xml-loader: malformed XML");
        let mut entries: Vec<CustomEntry> = Vec::new();
        // dict-custom.lisp:68 (dom:do-node-list (entry (dom:get-elements-by-tag-name parsed "entry")) ...)
        for entry in parsed
            .descendants()
            .filter(|n| n.is_element() && n.has_tag_name("entry"))
        {
            // dict-custom.lisp:69 (seq (ichiran/dict:node-text (dom:item (dom:get-elements-by-tag-name entry "ent_seq") 0)))
            let ent_seq_node = entry
                .descendants()
                .find(|n| n.is_element() && n.has_tag_name("ent_seq"))
                .expect("xml-loader: missing ent_seq element");
            let seq = node_text(ent_seq_node, None);
            // dict-custom.lisp:70 (nseq (handler-case (parse-integer seq) (error () seq)))
            let nseq = match seq.parse::<i32>() {
                Ok(n) => XmlEntrySeq::Int(n),
                Err(_) => XmlEntrySeq::String(seq.clone()),
            };
            // dict-custom.lisp:71 (push (make-xml-entry :seq nseq :content (rune-dom:create-document entry)) (entries loader))
            // ichiran stores a Document and serializes it later through cxml,
            // which prepends the XML 1.0 prolog.
            let range = entry.range();
            let xml_content = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}",
                &content[range],
            );
            entries.push(CustomEntry::Xml(XmlEntry {
                seq: nseq,
                content: xml_content,
            }));
        }
        // dict-custom.lisp:72 (setf (entries loader) (nreverse (entries loader)))
        self.base.entries = entries;
        Ok(())
    }

    pub async fn insert(&self, ctx: &KaniranContext) -> Result<(), sqlx::Error> {
        for entry in &self.base.entries {
            // dict-custom.lisp:75-80 (loop for entry in (entries loader) do (ichiran/dict::load-entry (xml-entry-content entry) :if-exists :skip :seq (xml-entry-seq entry) :conjugate-p t))
            let xml_entry = match entry {
                CustomEntry::Xml(x) => x,
                _ => panic!("xml-loader.insert: non-xml-entry in entries"),
            };
            let seq = match &xml_entry.seq {
                XmlEntrySeq::Int(n) => LoadEntrySeq::Int(*n),
                XmlEntrySeq::String(s) => LoadEntrySeq::Str(s),
            };
            load_entry(
                ctx,
                &xml_entry.content,
                LoadEntryIfExists::Skip,
                None,
                seq,
                true,
            )
            .await?;
        }
        Ok(())
    }
}

/// Port of `ichiran/custom:xml-entry` (`dict-custom.lisp:63`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlEntrySeq {
    Int(i32),
    String(String),
}

/// One parsed `<entry>` element from a custom XML source. `seq` is the
/// `<ent_seq>` text parsed to an integer if possible, otherwise the
/// raw string. `content` is the serialized XML of the single
/// `<entry>` element, ready for [`crate::dict::load_entry::load_entry`].
///
/// ```text
/// XmlEntry {
///     seq: XmlEntrySeq::Int(1234567),
///     content: "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<entry><ent_seq>1234567</ent_seq>...</entry>".to_string(),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct XmlEntry {
    pub seq: XmlEntrySeq,
    pub content: String,
}

/// Port of `ichiran/custom:csv-loader` (`dict-custom.lisp:82`).
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

/// Port of `ichiran/custom:municipality-csv` (`dict-custom.lisp:93`).
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
        assert_eq!(
            row.len(),
            5,
            "municipality-csv row arity (dict-custom.lisp:143)"
        );
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

/// Port of `ichiran/custom:municipality` (`dict-custom.lisp:140`).
///
/// In-memory record for one row of the municipalities CSV — a
/// prefecture, city, town, or village name with its kana reading and
/// romanized definition.
#[derive(Debug, Clone)]
pub struct Municipality {
    pub text: String,
    pub reading: String,
    pub definition: String,
    pub r#type: char,
    pub prefecture: Option<String>,
}

/// Port of `ichiran/custom:ward-csv` (`dict-custom.lisp:266`).
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
                let ct = city_text
                    .as_deref()
                    .expect("ward-csv: ward row before city row");
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
                let definition = format!(
                    "{}, {}",
                    romanize_municipality(&ward_text, &ward_reading, None),
                    cr_rom
                );
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

/// Port of `ichiran/custom:ward` (`dict-custom.lisp:269`).
///
/// In-memory record for one row of the wards CSV — a 区 subdivision of
/// a designated city, with its kana reading and romanized definition.
#[derive(Debug, Clone)]
pub struct Ward {
    pub text: String,
    pub reading: String,
    pub definition: String,
    pub city: String,
}
