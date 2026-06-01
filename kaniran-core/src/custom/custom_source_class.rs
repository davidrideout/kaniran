//! Port of `ichiran/custom:custom-source` (`dict-custom.lisp:54`).

use super::municipality_struct::Municipality;
use super::ward_struct::Ward;
use super::xml_entry_struct::XmlEntry;
use super::xml_loader_class::XmlLoader;
use super::municipality_csv_class::MunicipalityCsv;
use super::ward_csv_class::WardCsv;

/// Base class slots — `description` plus the polymorphic `entries`
/// vector that every subclass fills during [`super::slurp::slurp`].
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
