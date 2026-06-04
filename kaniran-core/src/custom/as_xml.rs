//! Port of `ichiran/custom:as-xml` (gf — `dict-custom.lisp:220`).

use super::as_xml_simple::as_xml_simple;
use super::custom_source_class::CustomEntry;

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

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::as-xml + dom:map-document), 2026-05-31.
    use super::*;
    use crate::custom::municipality_struct::Municipality;
    use crate::custom::ward_struct::Ward;

    #[test]
    #[should_panic(expected = "as-xml: no method for xml-entry")]
    fn as_xml_xml_entry_panics() {
        use crate::custom::xml_entry_struct::{XmlEntry, XmlEntrySeq};
        let _ = as_xml(&CustomEntry::Xml(XmlEntry {
            seq: XmlEntrySeq::Int(0),
            content: String::new(),
        }));
    }

    #[test]
    fn as_xml_municipality_fixture() {
        let m = Municipality {
            text: "東京".to_string(),
            reading: "とうきょう".to_string(),
            definition: "Tokyo Metropolis".to_string(),
            r#type: '都',
            prefecture: None,
        };
        assert_eq!(
            as_xml(&CustomEntry::Municipality(m)),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <entry><ent_seq></ent_seq><k_ele><keb>東京</keb></k_ele>\
             <r_ele><reb>とうきょう</reb></r_ele>\
             <sense><pos>n</pos><gloss>Tokyo Metropolis</gloss></sense></entry>",
        );
    }

    #[test]
    fn as_xml_ward_fixture() {
        let w = Ward {
            text: "中央".to_string(),
            reading: "ちゅうおう".to_string(),
            definition: "Chuo Ward, Tokyo".to_string(),
            city: "Tokyo".to_string(),
        };
        assert_eq!(
            as_xml(&CustomEntry::Ward(w)),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <entry><ent_seq></ent_seq><k_ele><keb>中央</keb></k_ele>\
             <r_ele><reb>ちゅうおう</reb></r_ele>\
             <sense><pos>n</pos><gloss>Chuo Ward, Tokyo</gloss></sense></entry>",
        );
    }
}
