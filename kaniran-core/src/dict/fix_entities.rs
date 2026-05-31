//! Port of `ichiran/dict:fix-entities` (`dict-load.lisp:161`).

use fancy_regex::{Captures, Regex};
use std::sync::OnceLock;

fn entity_decl_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"<!ENTITY\s+([A-Za-z_:][A-Za-z0-9._:\-]*)\s+(?:"([^"]*)"|'([^']*)')\s*>"#)
            .expect("fix_entities regex")
    })
}

pub fn fix_entities(source: &str) -> String {
    entity_decl_regex()
        .replace_all(source, |caps: &Captures<'_>| {
            let name = caps.get(1).expect("entity name").as_str();
            match name {
                "lt" | "gt" | "amp" | "apos" | "quot" => {
                    caps.get(0).expect("match").as_str().to_string()
                }
                _ => {
                    let quote_char = if caps.get(2).is_some() { '"' } else { '\'' };
                    format!("<!ENTITY {name} {quote_char}{name}{quote_char}>")
                }
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use roxmltree::{Document, ParsingOptions};

    // REPL fixture (.103, ichiran/dict::fix-entities + klacks serialize-element), 2026-05-31.
    #[test]
    fn fix_entities_rewrites_values_to_names_and_skips_predefined() {
        let xml = "<?xml version='1.0' encoding='UTF-8'?>
<!DOCTYPE JMdict [
<!ENTITY adj-i 'adjective (keiyoushi)'>
<!ENTITY n 'noun (common) (futsuumeishi)'>
<!ENTITY v5b 'Godan verb with bu ending'>
<!ENTITY v5b.alt \"alt form\">
<!ENTITY apos 'apostrophe-reserved'>
]>
<JMdict>
<entry><sense><pos>&n;</pos><pos>&v5b;</pos><pos>&v5b.alt;</pos><gloss>I&apos;m here</gloss></sense></entry>
</JMdict>";
        let fixed = fix_entities(xml);
        assert!(
            fixed.contains("<!ENTITY adj-i 'adj-i'>"),
            "expected adj-i rewrite, got:\n{fixed}"
        );
        assert!(
            fixed.contains("<!ENTITY n 'n'>"),
            "expected n rewrite, got:\n{fixed}"
        );
        assert!(
            fixed.contains("<!ENTITY v5b 'v5b'>"),
            "expected v5b rewrite, got:\n{fixed}"
        );
        assert!(
            fixed.contains("<!ENTITY v5b.alt \"v5b.alt\">"),
            "expected dotted name rewrite preserving quote style, got:\n{fixed}"
        );
        assert!(
            fixed.contains("<!ENTITY apos 'apostrophe-reserved'>"),
            "apos should be untouched, got:\n{fixed}"
        );

        let doc = Document::parse_with_options(
            &fixed,
            ParsingOptions { allow_dtd: true, ..Default::default() },
        )
        .unwrap();
        let entry = doc
            .root_element()
            .descendants()
            .find(|n| n.has_tag_name("entry"))
            .unwrap();
        assert_eq!(
            crate::dict::node_text::node_text(entry, None),
            "nv5bv5b.altI'm here"
        );
    }

    #[test]
    fn fix_entities_preserves_all_five_predefined_entities() {
        let xml = "<!DOCTYPE r [\
                   <!ENTITY lt 'X'>\
                   <!ENTITY gt 'X'>\
                   <!ENTITY amp 'X'>\
                   <!ENTITY apos 'X'>\
                   <!ENTITY quot 'X'>\
                   <!ENTITY other 'X'>\
                   ]>";
        let fixed = fix_entities(xml);
        for predefined in ["lt", "gt", "amp", "apos", "quot"] {
            let needle = format!("<!ENTITY {predefined} 'X'>");
            assert!(
                fixed.contains(&needle),
                "predefined {predefined} should be untouched"
            );
        }
        assert!(
            fixed.contains("<!ENTITY other 'other'>"),
            "non-predefined should be rewritten"
        );
    }
}
