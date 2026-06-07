use super::*;
use roxmltree::Document;

// --- first_node_text ---
#[test]
fn first_node_text_returns_none_on_empty_list() {
    let xml = "<root><x>42</x><x>99</x></root>";
    let doc = Document::parse(xml).unwrap();
    let empty: Vec<Node<'_, '_>> = doc
        .root_element()
        .descendants()
        .filter(|n| n.has_tag_name("y"))
        .collect();
    assert_eq!(first_node_text(&empty, |s| s.to_string()), None);
}

#[test]
fn first_node_text_returns_text_of_first_node() {
    let xml = "<root><x>42</x><x>99</x></root>";
    let doc = Document::parse(xml).unwrap();
    let xs: Vec<Node<'_, '_>> = doc
        .root_element()
        .descendants()
        .filter(|n| n.has_tag_name("x"))
        .collect();
    assert_eq!(
        first_node_text(&xs, |s| s.to_string()),
        Some(String::from("42"))
    );
}

#[test]
fn first_node_text_runs_parse_integer_wrapper() {
    let xml = "<root><x>42</x><x>99</x></root>";
    let doc = Document::parse(xml).unwrap();
    let xs: Vec<Node<'_, '_>> = doc
        .root_element()
        .descendants()
        .filter(|n| n.has_tag_name("x"))
        .collect();
    assert_eq!(
        first_node_text(&xs, |s| s.parse::<i32>().unwrap()),
        Some(42)
    );
}
