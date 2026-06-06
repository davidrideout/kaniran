//! Port of `ichiran/kanji:load-kanjidic` (`kanji.lisp:187`).
//!
//! Rebuilds the four kanjidic tables (`kanji`, `reading`, `okurigana`,
//! `meaning`) from a kanjidic2 XML dump. Drops & recreates the schema
//! via [`super::init_tables::init_tables`], iterates every `<character>`
//! child of the `<kanjidic2>` root, serializes each `<character>`
//! element back into an XML fragment, hands it to
//! [`super::load_kanji::load_kanji`], emits a progress line every
//! 500 entries, and finishes with `ANALYZE`.

use std::path::Path;

use roxmltree::{Document, Node, NodeType, ParsingOptions};

use super::init_tables::init_tables;
use super::load_kanji::load_kanji;
use crate::conn::kani_context::KaniranContext;

pub async fn load_kanjidic(ctx: &KaniranContext, path: &Path) -> Result<(), sqlx::Error> {
    // kanji.lisp:188 (init-tables)
    init_tables(ctx).await?;
    // kanji.lisp:189-191 (klacks:with-open-source (source (cxml:make-source path)) (klacks:find-element source "kanjidic2") …)
    let source = std::fs::read_to_string(path)
        .expect("load_kanjidic: failed to read kanjidic2 source");
    let parsed = Document::parse_with_options(
        &source,
        ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .expect("load_kanjidic: malformed kanjidic2 XML");
    let kanjidic2 = parsed
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("kanjidic2"))
        .expect("load_kanjidic: missing kanjidic2 root element");
    // kanji.lisp:192-197 (loop for cnt from 1 while (klacks:find-element source "character") …)
    let mut cnt: i32 = 1;
    for character_node in kanjidic2
        .children()
        .filter(|n| n.is_element() && n.has_tag_name("character"))
    {
        let content = serialize_character(character_node);
        load_kanji(ctx, &content).await?;
        if cnt % 500 == 0 {
            println!("{cnt} entries loaded");
        }
        cnt += 1;
    }
    // kanji.lisp:197 (finally (query "ANALYZE") (format t "~a entries total~%" cnt))
    sqlx::query("ANALYZE").execute(&ctx.pool).await?;
    println!("{cnt} entries total");
    Ok(())
}

/// Walks a `<character>` subtree and produces an XML string suitable
/// for [`super::load_kanji::load_kanji`] — mirrors
/// `klacks:serialize-element source (cxml:make-string-sink)`.
fn serialize_character(node: Node<'_, '_>) -> String {
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
