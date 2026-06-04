//! Port of `ichiran/custom:xml-loader` (`dict-custom.lisp:59`).

use std::path::PathBuf;

use roxmltree::Document;

use crate::dict::load_entry::{load_entry, LoadEntryIfExists, LoadEntrySeq};
use crate::dict::node_text::node_text;

use crate::conn::kani_context::KaniranContext;

use super::custom_source_class::{CustomEntry, CustomSource};
use super::xml_entry_struct::{XmlEntry, XmlEntrySeq};

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
