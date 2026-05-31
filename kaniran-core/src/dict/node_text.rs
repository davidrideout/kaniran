//! Port of `ichiran/dict:node-text` (`dict-load.lisp:18`).
//!
//! Concatenates text content of a DOM subtree

use roxmltree::{Node, NodeType};

pub fn node_text<'a, 'input>(
    node: Node<'a, 'input>,
    test: Option<&dyn Fn(Node<'a, 'input>) -> bool>,
) -> String {
    let mut values: Vec<String> = Vec::new();
    if test.map_or(true, |t| t(node)) {
        for child in node.children() {
            match child.node_type() {
                NodeType::Element => values.push(node_text(child, test)),
                NodeType::Text => {
                    if let Some(value) = child.text() {
                        values.push(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    values.concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use roxmltree::Document;

    // REPL fixtures (.103, ichiran/dict::node-text), 2026-05-31.
    #[test]
    fn node_text_jmdict_entry_concatenation() {
        let xml = "<entry><k_ele><keb>桜</keb></k_ele><r_ele><reb>さくら</reb></r_ele>\
                   <sense><pos>n</pos><gloss>cherry tree</gloss>\
                   <gloss>cherry blossom</gloss></sense></entry>";
        let doc = Document::parse(xml).unwrap();
        let entry = doc.root_element();
        assert_eq!(node_text(entry, None), "桜さくらncherry treecherry blossom");

        let sense = entry
            .descendants()
            .find(|n| n.has_tag_name("sense"))
            .unwrap();
        assert_eq!(node_text(sense, None), "ncherry treecherry blossom");

        let reb = entry
            .descendants()
            .find(|n| n.has_tag_name("reb"))
            .unwrap();
        assert_eq!(node_text(reb, None), "さくら");
    }

    #[test]
    fn node_text_preserves_whitespace_between_elements() {
        let xml = "<root><a> hello <b>world</b> end </a></root>";
        let doc = Document::parse(xml).unwrap();
        assert_eq!(node_text(doc.root_element(), None), " hello world end ");
    }

    #[test]
    fn node_text_predicate_gates_subtrees() {
        let xml = "<root><skip>NO</skip><keep>YES</keep></root>";
        let doc = Document::parse(xml).unwrap();
        let pred: &dyn Fn(Node<'_, '_>) -> bool = &|node: Node<'_, '_>| {
            node.node_type() == NodeType::Text || !node.has_tag_name("skip")
        };
        assert_eq!(node_text(doc.root_element(), Some(pred)), "YES");
    }

    #[test]
    fn node_text_blocking_predicate_returns_empty() {
        let xml = "<root>hi</root>";
        let doc = Document::parse(xml).unwrap();
        let pred: &dyn Fn(Node<'_, '_>) -> bool = &|_: Node<'_, '_>| false;
        assert_eq!(node_text(doc.root_element(), Some(pred)), "");
    }
}
