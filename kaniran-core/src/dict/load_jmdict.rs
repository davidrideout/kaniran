//! Port of `ichiran/dict:load-jmdict` (`dict-load.lisp:170`).
//!
//! Rebuilds the entry-package tables from a JMdict XML dump: drops &
//! recreates the schema via [`super::init_tables`], iterates every
//! `<entry>` in the source, hands each to [`super::load_entry`], and
//! (when requested) chains [`super::load_extras`] for the conjugation
//! / errata / custom-data pass.
//!
//! Diverges from the upstream lambda list `(&key path load-extras)`:
//! `path` is a required `&Path` (upstream defaults to the dynamic
//! `*jmdict-path*`, which is not a ported global per PORT_PLAN entry
//! 624 — config lives in `kaniran.toml`). `load_extras` keeps the
//! upstream default-on behavior. Replaces upstream `*connection*` with
//! `&KaniranContext` per [`crate::conn::kani_context`].
//!
//! The upstream uses CXML's klacks streaming pull-parser to walk the
//! source one `<entry>` at a time. The Rust port DOM-parses the whole
//! file via [`roxmltree::Document`] (kaniran's only XML dep) and
//! iterates the resulting tree; entry boundaries are recovered by
//! filtering top-level descendants. Each entry is re-serialized via
//! [`serialize_entry`] before being passed to [`super::load_entry`],
//! mirroring `klacks:serialize-element source (cxml:make-string-sink)`.

use std::path::Path;

use roxmltree::{Document, Node, NodeType, ParsingOptions};

use super::fix_entities::fix_entities;
use super::init_tables::init_tables;
use super::load_entry::{load_entry, LoadEntryIfExists, LoadEntrySeq};
use super::load_extras::load_extras;
use super::recalc_entry_stats_all::recalc_entry_stats_all;
use crate::conn::kani_context::KaniranContext;
use crate::custom::load_custom_data::LoadCustomDataError;

pub async fn load_jmdict(
    ctx: &KaniranContext,
    path: &Path,
    load_extras_p: bool,
) -> Result<(), LoadCustomDataError> {
    init_tables(ctx).await?;
    let source = std::fs::read_to_string(path)?;
    let fixed = fix_entities(&source);
    let parsed = Document::parse_with_options(
        &fixed,
        ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .expect("load_jmdict: malformed JMdict XML");
    // dict-load.lisp:174 (klacks:find-element source "JMdict")
    let jmdict = parsed
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("JMdict"))
        .expect("load_jmdict: missing JMdict root element");
    // dict-load.lisp:176-182 (loop ... while (klacks:find-element source "entry") ...)
    let mut cnt: i32 = 0;
    for entry_node in jmdict.children().filter(|n| n.is_element() && n.has_tag_name("entry")) {
        cnt += 1;
        let content = serialize_entry(entry_node);
        load_entry(
            ctx,
            &content,
            LoadEntryIfExists::None,
            None,
            LoadEntrySeq::None,
            true,
        )
        .await?;
        if cnt % 1000 == 0 {
            println!("{cnt} entries loaded");
        }
    }
    recalc_entry_stats_all(ctx).await?;
    sqlx::query("ANALYZE").execute(&ctx.pool).await?;
    println!("{cnt} entries total");
    if load_extras_p {
        load_extras(ctx).await?;
    }
    Ok(())
}

/// Walks an `<entry>` subtree and produces an XML string suitable for
/// [`super::load_entry`] — every entity reference has already been
/// resolved to its short name by [`fix_entities`], so the output is
/// standalone XML with no DTD attached.
fn serialize_entry(node: Node<'_, '_>) -> String {
    let mut out = String::new();
    write_node(node, &mut out);
    out
}

fn write_node(node: Node<'_, '_>, out: &mut String) {
    match node.node_type() {
        NodeType::Element => {
            let name = node.tag_name().name();
            out.push('<');
            out.push_str(name);
            for attr in node.attributes() {
                out.push(' ');
                out.push_str(attr.name());
                out.push_str("=\"");
                write_escaped_attr(attr.value(), out);
                out.push('"');
            }
            if node.first_child().is_none() {
                out.push_str("/>");
                return;
            }
            out.push('>');
            for child in node.children() {
                write_node(child, out);
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        NodeType::Text => {
            if let Some(text) = node.text() {
                write_escaped_text(text, out);
            }
        }
        _ => {}
    }
}

fn write_escaped_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            _ => out.push(c),
        }
    }
}

fn write_escaped_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dict::node_text::node_text;

    // REPL fixture (.103, ichiran/dict::fix-entities → klacks serialize-element on a JMdict entry), 2026-05-31.
    // Input mirrors the JMdict shape: DOCTYPE + entity decls, one <entry> with kanji, kana, and a sense block.
    #[test]
    fn serialize_entry_roundtrips_entities_and_nested_text() {
        let xml = "<?xml version='1.0' encoding='UTF-8'?>\
<!DOCTYPE JMdict [\
<!ENTITY n 'noun (common) (futsuumeishi)'>\
<!ENTITY adj-na 'adjectival nouns or quasi-adjectives (keiyodoshi)'>\
]>\
<JMdict>\
<entry>\
<ent_seq>1582710</ent_seq>\
<k_ele><keb>陶器市</keb></k_ele>\
<r_ele><reb>とうきいち</reb></r_ele>\
<sense><pos>&n;</pos><pos>&adj-na;</pos><gloss>pottery fair</gloss></sense>\
</entry>\
</JMdict>";
        let fixed = fix_entities(xml);
        let doc = Document::parse_with_options(
            &fixed,
            ParsingOptions { allow_dtd: true, ..Default::default() },
        )
        .unwrap();
        let entry = doc
            .descendants()
            .find(|n| n.is_element() && n.has_tag_name("entry"))
            .unwrap();
        let serialized = serialize_entry(entry);
        assert_eq!(
            serialized,
            "<entry>\
<ent_seq>1582710</ent_seq>\
<k_ele><keb>陶器市</keb></k_ele>\
<r_ele><reb>とうきいち</reb></r_ele>\
<sense><pos>n</pos><pos>adj-na</pos><gloss>pottery fair</gloss></sense>\
</entry>"
        );
        // re-parse: the serialized form must stand alone (no DTD attached)
        let reparsed = Document::parse(&serialized).expect("reparse standalone");
        let reparsed_entry = reparsed
            .descendants()
            .find(|n| n.has_tag_name("entry"))
            .expect("reparsed entry");
        let ent_seq = reparsed_entry
            .descendants()
            .find(|n| n.has_tag_name("ent_seq"))
            .unwrap();
        assert_eq!(node_text(ent_seq, None), "1582710");
        let keb = reparsed_entry
            .descendants()
            .find(|n| n.has_tag_name("keb"))
            .unwrap();
        assert_eq!(node_text(keb, None), "陶器市");
        let reb = reparsed_entry
            .descendants()
            .find(|n| n.has_tag_name("reb"))
            .unwrap();
        assert_eq!(node_text(reb, None), "とうきいち");
        let pos_texts: Vec<String> = reparsed_entry
            .descendants()
            .filter(|n| n.has_tag_name("pos"))
            .map(|n| node_text(n, None))
            .collect();
        assert_eq!(pos_texts, vec!["n".to_string(), "adj-na".to_string()]);
        let gloss = reparsed_entry
            .descendants()
            .find(|n| n.has_tag_name("gloss"))
            .unwrap();
        assert_eq!(node_text(gloss, None), "pottery fair");
    }

    // REPL fixture (.103, ichiran/dict::fix-entities → klacks serialize-element with embedded "<" and "&"), 2026-05-31.
    // Metacharacters must escape to &lt; / &amp;, and re-parse must yield the original text exactly.
    #[test]
    fn serialize_entry_escapes_xml_metacharacters_in_text() {
        let xml = "<entry><sense><gloss>a &lt; b &amp; c</gloss></sense></entry>";
        let doc = Document::parse(xml).unwrap();
        let entry = doc.root_element();
        let serialized = serialize_entry(entry);
        assert_eq!(
            serialized,
            "<entry><sense><gloss>a &lt; b &amp; c</gloss></sense></entry>"
        );
        let reparsed = Document::parse(&serialized).expect("reparse standalone");
        let gloss = reparsed
            .descendants()
            .find(|n| n.has_tag_name("gloss"))
            .unwrap();
        assert_eq!(node_text(gloss, None), "a < b & c");
    }

    // Empty elements: JMdict spells both <re_nokanji/> and (legally) the explicit-end-tag form; both must collapse to self-closing on output.
    #[test]
    fn serialize_entry_emits_self_closing_for_empty_elements() {
        let self_closing = "<entry><r_ele><reb>カラ</reb><re_nokanji/></r_ele></entry>";
        let doc = Document::parse(self_closing).unwrap();
        let serialized = serialize_entry(doc.root_element());
        assert_eq!(
            serialized,
            "<entry><r_ele><reb>カラ</reb><re_nokanji/></r_ele></entry>"
        );
        let explicit = "<entry><r_ele><reb>カラ</reb><re_nokanji></re_nokanji></r_ele></entry>";
        let doc2 = Document::parse(explicit).unwrap();
        let serialized2 = serialize_entry(doc2.root_element());
        assert_eq!(
            serialized2,
            "<entry><r_ele><reb>カラ</reb><re_nokanji/></r_ele></entry>"
        );
    }
}
