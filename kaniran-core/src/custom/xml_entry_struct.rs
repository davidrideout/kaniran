//! Port of `ichiran/custom:xml-entry` (`dict-custom.lisp:63`).

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
///     content: "<entry><ent_seq>1234567</ent_seq>...</entry>".to_string(),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct XmlEntry {
    pub seq: XmlEntrySeq,
    pub content: String,
}
