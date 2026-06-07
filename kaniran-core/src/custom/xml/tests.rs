use super::*;
use crate::custom::types::{Municipality, Ward};

// --- as_xml ---

#[test]
#[should_panic(expected = "as-xml: no method for xml-entry")]
fn as_xml_xml_entry_panics() {
    use crate::custom::types::{XmlEntry, XmlEntrySeq};
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

// --- as_xml_simple ---

#[test]
fn as_xml_simple_fixtures() {
    assert_eq!(
        as_xml_simple("とうきょう", "とうきょう", "Tokyo"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <entry><ent_seq></ent_seq><r_ele><reb>とうきょう</reb></r_ele>\
         <sense><pos>n</pos><gloss>Tokyo</gloss></sense></entry>",
    );
    assert_eq!(
        as_xml_simple("コーヒー", "こーひー", "coffee"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <entry><ent_seq></ent_seq><r_ele><reb>コーヒー</reb></r_ele>\
         <sense><pos>n</pos><gloss>coffee</gloss></sense></entry>",
    );
    assert_eq!(
        as_xml_simple("東京", "とうきょう", "Tokyo Metropolis"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <entry><ent_seq></ent_seq><k_ele><keb>東京</keb></k_ele>\
         <r_ele><reb>とうきょう</reb></r_ele>\
         <sense><pos>n</pos><gloss>Tokyo Metropolis</gloss></sense></entry>",
    );
    assert_eq!(
        as_xml_simple("横浜", "よこはま", "Yokohama (city)"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <entry><ent_seq></ent_seq><k_ele><keb>横浜</keb></k_ele>\
         <r_ele><reb>よこはま</reb></r_ele>\
         <sense><pos>n</pos><gloss>Yokohama (city)</gloss></sense></entry>",
    );
    assert_eq!(
        as_xml_simple("鎌倉市", "かまくらし", "Kamakura (city)"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <entry><ent_seq></ent_seq><k_ele><keb>鎌倉市</keb></k_ele>\
         <r_ele><reb>かまくらし</reb></r_ele>\
         <sense><pos>n</pos><gloss>Kamakura (city)</gloss></sense></entry>",
    );
}

// XML text content escapes only <, >, &; ' and " stay literal.
#[test]
fn as_xml_simple_escapes_metachars() {
    let out = as_xml_simple("A&B", "えーびー", "A < B & C > 'apos' \"quot\"");
    assert!(out.contains("<keb>A&amp;B</keb>"), "got {out}");
    assert!(
        out.contains("<gloss>A &lt; B &amp; C &gt; 'apos' \"quot\"</gloss>"),
        "got {out}",
    );
}

// An apostrophe in the romanized name stays literal, not encoded as &apos;.
#[test]
fn as_xml_simple_apostrophe_is_literal() {
    assert_eq!(
        as_xml_simple("南陽市", "なんようし", "Nan'Yo (city), Yamagata Prefecture"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <entry><ent_seq></ent_seq><k_ele><keb>南陽市</keb></k_ele>\
         <r_ele><reb>なんようし</reb></r_ele>\
         <sense><pos>n</pos><gloss>Nan'Yo (city), Yamagata Prefecture</gloss></sense></entry>",
    );
}
