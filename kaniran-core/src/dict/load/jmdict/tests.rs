use super::*;
use crate::conn::kani_context::KaniranContext;
use crate::dict::load::jmdict::{init_tables, node_text};
use roxmltree::{Document, ParsingOptions};

// --- node_text ---
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

    let reb = entry.descendants().find(|n| n.has_tag_name("reb")).unwrap();
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
    let pred: &dyn Fn(Node<'_, '_>) -> bool =
        &|node: Node<'_, '_>| node.node_type() == NodeType::Text || !node.has_tag_name("skip");
    assert_eq!(node_text(doc.root_element(), Some(pred)), "YES");
}

#[test]
fn node_text_blocking_predicate_returns_empty() {
    let xml = "<root>hi</root>";
    let doc = Document::parse(xml).unwrap();
    let pred: &dyn Fn(Node<'_, '_>) -> bool = &|_: Node<'_, '_>| false;
    assert_eq!(node_text(doc.root_element(), Some(pred)), "");
}

// --- sense_exists_p ---
fn raw(ord: i32, gloss: &str, props: &[(&str, &[&str])]) -> RawSense {
    RawSense {
        ord,
        gloss: gloss.to_string(),
        props: props
            .iter()
            .map(|(tag, vals)| {
                (
                    (*tag).to_string(),
                    vals.iter().map(|s| (*s).to_string()).collect(),
                )
            })
            .collect(),
    }
}

fn sv(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn single_sense_exact_match() {
    // One sense: gloss "Japan", pos "n".
    let senses = vec![raw(0, "Japan", &[("pos", &["n"])])];
    assert!(sense_exists_p(&senses, &sv(&["n"]), &sv(&["Japan"])));
}

#[test]
fn single_sense_mismatches_and_empty_inputs() {
    let senses = vec![raw(0, "Japan", &[("pos", &["n"])])];
    let cases: &[(&str, &[&str], &[&str], bool)] = &[
        ("diff-gloss", &["n"], &["Korea"], false),
        ("diff-pos", &["vt"], &["Japan"], false),
        ("empty-glosses", &["n"], &[], false),
        ("empty-positions", &[], &["Japan"], false),
    ];
    for (label, pos, gl, expected) in cases {
        assert_eq!(
            sense_exists_p(&senses, &sv(pos), &sv(gl)),
            *expected,
            "{label}",
        );
    }
}

#[test]
fn multi_pos_value_lists_must_match_in_order() {
    // Two senses whose `pos` value-lists are in opposite orders.
    let senses = vec![
        raw(
            0,
            "to treat; to handle; to deal with",
            &[("stagk", &["遇う"]), ("pos", &["v5u", "vt"])],
        ),
        raw(
            1,
            "to arrange; to decorate (with); to adorn (with); to dress (with); to garnish (with)",
            &[("pos", &["vt", "v5u"])],
        ),
    ];
    assert!(sense_exists_p(
        &senses,
        &sv(&["v5u", "vt"]),
        &sv(&["to treat", "to handle", "to deal with"]),
    ));
    assert!(sense_exists_p(
        &senses,
        &sv(&["vt", "v5u"]),
        &sv(&[
            "to arrange",
            "to decorate (with)",
            "to adorn (with)",
            "to dress (with)",
            "to garnish (with)",
        ]),
    ));
    // first sense's joined gloss with sense-2's pos order — no match
    assert!(!sense_exists_p(
        &senses,
        &sv(&["vt", "v5u"]),
        &sv(&["to treat", "to handle", "to deal with"]),
    ));
}

#[test]
fn second_sense_with_no_props_inherits_prior_pos() {
    // Two senses: "Tokyo" with pos "n", then "Tokyo Metropolis" with no props.
    let senses = vec![
        raw(0, "Tokyo", &[("pos", &["n"])]),
        raw(1, "Tokyo Metropolis", &[]),
    ];
    let cases: &[(&str, &[&str], &[&str], bool)] = &[
        ("sense1-match", &["n"], &["Tokyo"], true),
        ("sense2-rpos-fallback", &["n"], &["Tokyo Metropolis"], true),
        ("sense2-wrong-pos", &["v5u"], &["Tokyo Metropolis"], false),
        ("sense1-wrong-pos", &["vt"], &["Tokyo"], false),
    ];
    for (label, pos, gl, expected) in cases {
        assert_eq!(
            sense_exists_p(&senses, &sv(pos), &sv(gl)),
            *expected,
            "{label}",
        );
    }
}

#[test]
fn first_sense_without_pos_does_not_match_pos_request() {
    // A sense with no pos can't match a non-empty pos request even when
    // its gloss matches.
    let senses = vec![raw(0, "a", &[]), raw(1, "b", &[("pos", &["n"])])];
    assert!(sense_exists_p(&senses, &sv(&["n"]), &sv(&["b"])));
    assert!(!sense_exists_p(&senses, &sv(&["n"]), &sv(&["a"])));
}

#[test]
fn glosses_joined_with_separator_before_compare() {
    // The input glosses are joined with "; " before comparing against the
    // stored gloss string.
    let senses = vec![raw(0, "Japan; Land of the Rising Sun", &[("pos", &["n"])])];
    assert!(sense_exists_p(
        &senses,
        &sv(&["n"]),
        &sv(&["Japan", "Land of the Rising Sun"]),
    ));
    // A single-element input whose value already contains the separator
    // joins to itself (no separator inserted), so it also matches.
    assert!(sense_exists_p(
        &senses,
        &sv(&["n"]),
        &sv(&["Japan; Land of the Rising Sun"]),
    ));
}

#[test]
fn empty_inputs_match_when_sense_is_also_empty() {
    // A sense with an empty gloss and no props matches an empty pos /
    // empty gloss request.
    let senses = vec![raw(0, "", &[])];
    assert!(sense_exists_p(&senses, &[], &[]));
}

#[test]
fn empty_sense_list_never_matches() {
    assert!(!sense_exists_p(&[], &sv(&["n"]), &sv(&["foo"])));
}

// --- init_tables ---
// Issues DDL (TRUNCATE), so refuses to touch any database that
// hasn't been explicitly named via `KANIRAN_TEST_DATABASE_URL`.
// Requires `--test-threads=1` since parallel runs share the same
// database.

#[tokio::test]
#[ignore = "DDL test; requires KANIRAN_TEST_DATABASE_URL"]
async fn init_tables_is_idempotent() {
    let ctx = KaniranContext::pool_only_test_ctx().await;
    init_tables(&ctx).await.expect("first run");
    init_tables(&ctx)
        .await
        .expect("second run must be idempotent");
}

// --- next_seq ---
#[tokio::test]
#[ignore = "DB test; requires KANIRAN_TEST_DATABASE_URL"]
async fn next_seq_returns_max_plus_one() {
    let ctx = KaniranContext::pool_only_test_ctx().await;
    init_tables(&ctx).await.unwrap();
    // next_seq returns the maximum existing seq plus one.
    sqlx::query(
        "INSERT INTO entry (seq, content, root_p, n_kanji, n_kana, primary_nokanji) \
         VALUES (1000000, '', TRUE, 0, 0, FALSE), \
                (1000042, '', TRUE, 0, 0, FALSE)",
    )
    .execute(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(next_seq(&ctx).await.unwrap(), 1000043);
}

// --- fix_entities ---
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
        ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap();
    let entry = doc
        .root_element()
        .descendants()
        .find(|n| n.has_tag_name("entry"))
        .unwrap();
    assert_eq!(
        crate::dict::load::jmdict::node_text(entry, None),
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

// --- load_jmdict ---
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
        ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap();
    let entry = doc
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("entry"))
        .unwrap();
    let serialized = serialize_entry(entry);
    assert_eq!(
        serialized,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<entry>\
<ent_seq>1582710</ent_seq>\
<k_ele><keb>陶器市</keb></k_ele>\
<r_ele><reb>とうきいち</reb></r_ele>\
<sense><pos>n</pos><pos>adj-na</pos>\
<gloss xml:lang=\"eng\">pottery fair</gloss></sense>\
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

// Metacharacters must escape to &lt; / &amp;, and re-parse must yield the original text exactly.
#[test]
fn serialize_entry_escapes_xml_metacharacters_in_text() {
    let xml = "<entry><sense><gloss>a &lt; b &amp; c</gloss></sense></entry>";
    let doc = Document::parse(xml).unwrap();
    let entry = doc.root_element();
    let serialized = serialize_entry(entry);
    assert_eq!(
        serialized,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<entry><sense><gloss xml:lang=\"eng\">a &lt; b &amp; c</gloss></sense></entry>"
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
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<entry><r_ele><reb>カラ</reb><re_nokanji/></r_ele></entry>"
    );
    let explicit = "<entry><r_ele><reb>カラ</reb><re_nokanji></re_nokanji></r_ele></entry>";
    let doc2 = Document::parse(explicit).unwrap();
    let serialized2 = serialize_entry(doc2.root_element());
    assert_eq!(
        serialized2,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<entry><r_ele><reb>カラ</reb><re_nokanji/></r_ele></entry>"
    );
}

// Pins three serializer behaviors in the byte output: the XML prolog,
// the DTD-default xml:lang on gloss, and no trailing newline after </entry>.
#[test]
fn serialize_entry_matches_ref_db_byte_shape() {
    let xml = "<?xml version='1.0' encoding='UTF-8'?>\
<!DOCTYPE JMdict [\
]>\
<JMdict>\n\
<entry>\n\
<ent_seq>1000000</ent_seq>\n\
<r_ele>\n\
<reb>ヽ</reb>\n\
</r_ele>\n\
<sense>\n\
<pos>unc</pos>\n\
<xref>一の字点</xref>\n\
<gloss>repetition mark in katakana</gloss>\n\
</sense>\n\
</entry>\n\
</JMdict>";
    let fixed = fix_entities(xml);
    let doc = Document::parse_with_options(
        &fixed,
        ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .unwrap();
    let entry = doc
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("entry"))
        .unwrap();
    assert_eq!(
        serialize_entry(entry),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<entry>\n\
<ent_seq>1000000</ent_seq>\n\
<r_ele>\n\
<reb>ヽ</reb>\n\
</r_ele>\n\
<sense>\n\
<pos>unc</pos>\n\
<xref>一の字点</xref>\n\
<gloss xml:lang=\"eng\">repetition mark in katakana</gloss>\n\
</sense>\n\
</entry>"
    );
}

// DTD-default xml:lang must come before any source-specified attribute
// on the same element: `<gloss xml:lang="eng" g_type="expl">`.
#[test]
fn serialize_entry_injects_xml_lang_before_other_attrs() {
    let xml = "<entry><sense>\
<gloss g_type=\"expl\">explanation</gloss>\
<gloss xml:lang=\"deu\">Erklärung</gloss>\
</sense></entry>";
    let doc = Document::parse(xml).unwrap();
    let serialized = serialize_entry(doc.root_element());
    assert_eq!(
        serialized,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<entry><sense>\
<gloss xml:lang=\"eng\" g_type=\"expl\">explanation</gloss>\
<gloss xml:lang=\"deu\">Erklärung</gloss>\
</sense></entry>"
    );
}
