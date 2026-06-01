//! Port of `ichiran/custom:as-xml-simple` (`dict-custom.lisp:223`).
//!
//! Lisp builds a `rune-dom` document; the Rust port returns the
//! serialized form because `load_entry` re-parses strings.

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;

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
    // dict-custom.lisp:225-244 (cxml:with-element "entry" (cxml:with-element "ent_seq" (cxml:text "")) ... (cxml:with-element "sense" (cxml:with-element "pos" (cxml:text "n")) (cxml:with-element "gloss" (cxml:attribute "xml:lang" "eng") (cxml:text definition))))
    format!(
        "<entry><ent_seq></ent_seq>{body}<sense><pos>n</pos><gloss xml:lang=\"eng\">{}</gloss></sense></entry>",
        xml_escape(definition),
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::as-xml-simple +
    //! dom:map-document), 2026-05-31. The cxml serializer drops the
    //! `xml:lang="eng"` attribute; the Rust port emits it because
    //! `load-entry` re-parses and ignores it.
    use super::*;

    #[test]
    fn as_xml_simple_fixtures() {
        assert_eq!(
            as_xml_simple("とうきょう", "とうきょう", "Tokyo"),
            "<entry><ent_seq></ent_seq><r_ele><reb>とうきょう</reb></r_ele>\
             <sense><pos>n</pos><gloss xml:lang=\"eng\">Tokyo</gloss></sense></entry>",
        );
        assert_eq!(
            as_xml_simple("コーヒー", "こーひー", "coffee"),
            "<entry><ent_seq></ent_seq><r_ele><reb>コーヒー</reb></r_ele>\
             <sense><pos>n</pos><gloss xml:lang=\"eng\">coffee</gloss></sense></entry>",
        );
        assert_eq!(
            as_xml_simple("東京", "とうきょう", "Tokyo Metropolis"),
            "<entry><ent_seq></ent_seq><k_ele><keb>東京</keb></k_ele>\
             <r_ele><reb>とうきょう</reb></r_ele>\
             <sense><pos>n</pos><gloss xml:lang=\"eng\">Tokyo Metropolis</gloss></sense></entry>",
        );
        assert_eq!(
            as_xml_simple("横浜", "よこはま", "Yokohama (city)"),
            "<entry><ent_seq></ent_seq><k_ele><keb>横浜</keb></k_ele>\
             <r_ele><reb>よこはま</reb></r_ele>\
             <sense><pos>n</pos><gloss xml:lang=\"eng\">Yokohama (city)</gloss></sense></entry>",
        );
        assert_eq!(
            as_xml_simple("鎌倉市", "かまくらし", "Kamakura (city)"),
            "<entry><ent_seq></ent_seq><k_ele><keb>鎌倉市</keb></k_ele>\
             <r_ele><reb>かまくらし</reb></r_ele>\
             <sense><pos>n</pos><gloss xml:lang=\"eng\">Kamakura (city)</gloss></sense></entry>",
        );
    }

    #[test]
    fn as_xml_simple_escapes_metachars() {
        let out = as_xml_simple("A&B", "えーびー", "A < B & C > 'apos' \"quot\"");
        assert!(out.contains("<keb>A&amp;B</keb>"), "got {out}");
        assert!(
            out.contains(
                "<gloss xml:lang=\"eng\">A &lt; B &amp; C &gt; &apos;apos&apos; &quot;quot&quot;</gloss>"
            ),
            "got {out}",
        );
    }
}
