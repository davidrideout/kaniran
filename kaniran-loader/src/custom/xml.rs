use super::types::CustomEntry;
use kaniran_core::characters::char_class::CharClass;
use kaniran_core::characters::char_class::test_word;

/// Port of `ichiran/custom:as-xml` (gf — `dict-custom.lisp:220`).
pub fn as_xml(entry: &CustomEntry) -> String {
    match entry {
        // dict-custom.lisp:246-247 (defmethod as-xml ((entry municipality)) (as-xml-simple (municipality-text entry) (municipality-reading entry) (municipality-definition entry)))
        CustomEntry::Municipality(m) => as_xml_simple(&m.text, &m.reading, &m.definition),
        // dict-custom.lisp:302-303 (defmethod as-xml ((entry ward)) (as-xml-simple (ward-text entry) (ward-reading entry) (ward-definition entry)))
        CustomEntry::Ward(w) => as_xml_simple(&w.text, &w.reading, &w.definition),
        CustomEntry::Xml(_) => {
            panic!("as-xml: no method for xml-entry (dict-custom.lisp:220)")
        }
    }
}

/// Port of `ichiran/custom:as-xml-simple` (`dict-custom.lisp:223`).
///
/// Builds a JMdict-style XML `<entry>` string for a custom entry from
/// its text, reading, and definition.
pub fn as_xml_simple(text: &str, reading: &str, definition: &str) -> String {
    // dict-custom.lisp:228 (cond ((test-word text :kana) ...) (t ...))
    let body = if test_word(text, CharClass::Kana) {
        // dict-custom.lisp:229-231 (cxml:with-element "r_ele" (cxml:with-element "reb" (cxml:text text)))
        format!("<r_ele><reb>{}</reb></r_ele>", xml_escape(text))
    } else {
        // dict-custom.lisp:232-238 (cxml:with-element "k_ele" (cxml:with-element "keb" (cxml:text text))) (cxml:with-element "r_ele" (cxml:with-element "reb" (cxml:text reading)))
        format!(
            "<k_ele><keb>{}</keb></k_ele><r_ele><reb>{}</reb></r_ele>",
            xml_escape(text),
            xml_escape(reading),
        )
    };
    // ichiran serializes each entry through cxml's string sink, which prepends
    // the XML 1.0 prolog and drops the `xml:lang="eng"` attribute (no DTD here
    // to default it).
    // dict-custom.lisp:225-244 (cxml:with-element "entry" (cxml:with-element "ent_seq" (cxml:text "")) ... (cxml:with-element "sense" (cxml:with-element "pos" (cxml:text "n")) (cxml:with-element "gloss" (cxml:attribute "xml:lang" "eng") (cxml:text definition))))
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<entry><ent_seq></ent_seq>{body}<sense><pos>n</pos><gloss>{}</gloss></sense></entry>",
        xml_escape(definition),
    )
}

// Only <, >, & need escaping in XML text content; ' and " are left literal
// to match ichiran's serializer (verified on .103).
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests;
