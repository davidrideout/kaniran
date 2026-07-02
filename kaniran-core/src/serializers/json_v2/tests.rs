//! Logic tests for the v2 reshaping helpers, plus DB-backed tests that
//! render real sentences against the configured backend and assert the
//! tokens + entries contract (`docs/output_formats.md`). Expected values
//! are real pipeline output / ichiran_latest rows.

use super::*;
use crate::core::methods::{hepburn_traditional, RomanizationMethod};
use serde_json::Value;

/// A conjugation step with a form name; `pos` is irrelevant to display.
fn step(form: &str, negative: bool, formal: bool) -> V2Step {
    V2Step {
        form: form.to_owned(),
        pos: "v1".to_owned(),
        negative,
        formal,
    }
}

#[test]
fn build_display_single_step_is_just_the_form() {
    let steps = [step("Continuative (~i)", false, false)];
    assert_eq!(build_display(&steps), "Continuative (~i)");
}

#[test]
fn build_display_via_chain_appends_deeper_steps_reversed() {
    // 食べさせられ: steps run root→surface [Causative-Passive, Continuative],
    // and display is surface-first, deeper trailing after " via ".
    let steps = [
        step("Causative-Passive", false, false),
        step("Continuative (~i)", false, false),
    ];
    assert_eq!(
        build_display(&steps),
        "Continuative (~i) via Causative-Passive"
    );
}

#[test]
fn build_display_marks_negative_and_formal_on_the_surface_step() {
    assert_eq!(
        build_display(&[step("Past (~ta)", true, false)]),
        "Past (~ta), negative"
    );
    assert_eq!(
        build_display(&[step("Past (~ta)", false, true)]),
        "Past (~ta), formal"
    );
    assert_eq!(build_display(&[]), "");
}

#[test]
fn split_reading_handles_composite_kana_only_and_none() {
    // The conjugation base reading arrives as "食べる 【たべる】"; たい has
    // no kanji head, so it is both the form and the reading.
    let (base_form, base_reading) = split_reading(Some("食べる 【たべる】".to_owned()));
    assert_eq!(base_form.as_deref(), Some("食べる"));
    assert_eq!(base_reading.as_deref(), Some("たべる"));

    let (base_form, base_reading) = split_reading(Some("たい".to_owned()));
    assert_eq!(base_form.as_deref(), Some("たい"));
    assert_eq!(base_reading.as_deref(), Some("たい"));

    assert_eq!(split_reading(None), (None, None));
}

#[test]
fn strip_marks_drops_zero_width_markers() {
    // The は particle reading carries a leading zero-width non-joiner.
    assert_eq!(strip_marks("\u{200c}は"), "は");
    assert_eq!(strip_marks("こんにち\u{200c}は"), "こんにちは");
}

#[test]
fn parse_common_tags_splits_bracketed_priority_tags() {
    // kanji_text.common_tags for 食べる, straight from ichiran_latest.
    assert_eq!(
        parse_common_tags("[ichi1][news2][nf25]"),
        ["ichi1", "news2", "nf25"]
    );
    assert_eq!(parse_common_tags(""), Vec::<String>::new());
}

#[test]
fn split_glosses_inverts_the_join_and_keeps_empty_empty() {
    assert_eq!(
        split_glosses("to live on (e.g. a salary); to live off; to subsist on"),
        [
            "to live on (e.g. a salary)",
            "to live off",
            "to subsist on"
        ]
    );
    assert_eq!(split_glosses(""), Vec::<String>::new());
    assert_eq!(strip_value_prefix("Value: 35"), "35");
}

// --- DB-backed rendering tests ---

fn method() -> KaniRomanizeMethod<'static> {
    KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
}

fn render_value_with(input: &str, options: V2Options) -> Value {
    let ctx = crate::test_support::shared_ctx();
    let document = render(&ctx, input, method(), 1, options).unwrap();
    serde_json::from_str(&document).unwrap()
}

fn render_value(input: &str) -> Value {
    render_value_with(input, V2Options::default())
}

fn token_texts(value: &Value) -> Vec<String> {
    value["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|token| token["text"].as_str().unwrap().to_owned())
        .collect()
}

/// Every entry id referenced anywhere in the tokens must resolve in the
/// entries table.
fn assert_entry_integrity(value: &Value) {
    let entries = value["entries"].as_object().unwrap();
    let check = |spot: &Value| {
        if let Some(id) = spot["entry"].as_i64() {
            assert!(
                entries.contains_key(&id.to_string()),
                "entry {id} referenced but not in entries"
            );
        }
        for analysis in spot["conjugation"].as_array().into_iter().flatten() {
            let id = analysis["entry"].as_i64().unwrap();
            assert!(
                entries.contains_key(&id.to_string()),
                "analysis entry {id} referenced but not in entries"
            );
        }
    };
    for token in value["tokens"].as_array().unwrap() {
        check(token);
        for alternative in token["alternatives"].as_array().into_iter().flatten() {
            check(alternative);
        }
    }
}

#[test]
fn verbatim_tokens_reconstruct_the_input() {
    // 、 and 。 normalize to ", " / ". " in the romanization, but token text
    // must quote the input; concatenating tokens[].text reproduces it exactly.
    let value = render_value("はい、そうです。");
    assert_eq!(token_texts(&value).concat(), "はい、そうです。");

    let tokens = value["tokens"].as_array().unwrap();
    let comma = tokens
        .iter()
        .find(|token| token["text"] == "、")
        .expect("、 gap token");
    assert_eq!(comma["romanization"], ", ");
    assert_eq!(comma["gap"], true);
    assert!(comma.get("reading").is_none());
    assert!(comma.get("entry").is_none());
    assert!(comma.get("score").is_none());

    // Full-width digits: dictionary matching runs on the normalized "10本",
    // the token quotes the input, and the counter carries the bare value.
    let value = render_value("１０本ください。");
    assert_eq!(token_texts(&value).concat(), "１０本ください。");
    let counter_token = &value["tokens"][0];
    assert_eq!(counter_token["text"], "１０本");
    assert_eq!(counter_token["counter"]["value"], "10");
}

#[test]
fn compound_members_share_an_id_and_carry_suffix_and_analysis() {
    // 食べている = 食べて + いる, one suffix compound.
    let value = render_value("食べている");
    let tokens = value["tokens"].as_array().unwrap();
    assert_eq!(token_texts(&value), ["食べて", "いる"]);
    assert_eq!(tokens[0]["compound"], 1);
    assert_eq!(tokens[1]["compound"], 1);

    let taberu = &tokens[0];
    assert_eq!(taberu["entry"], 1358280);
    let analysis = &taberu["conjugation"][0];
    assert_eq!(analysis["entry"], 1358280);
    assert_eq!(analysis["base_form"], "食べる");
    assert_eq!(analysis["base_reading"], "たべる");
    assert_eq!(analysis["description"], "Conjunctive (~te)");
    assert_eq!(analysis["steps"][0]["form"], "Conjunctive (~te)");
    assert_eq!(analysis["steps"][0]["pos"], "v1");
    // negative/formal are omitted when false, reading_matched when true.
    assert!(analysis["steps"][0].get("negative").is_none());
    assert!(analysis.get("reading_matched").is_none());

    let iru = &tokens[1];
    assert_eq!(iru["entry"], 1577980);
    assert_eq!(iru["suffix"]["class"], "iru");
    assert_eq!(
        iru["suffix"]["description"],
        "indicates continuing action (to be ...ing)"
    );

    // Surface-form ruby: 食 reads た, べて passes through.
    let furigana = taberu["furigana"].as_array().unwrap();
    assert_eq!(furigana.len(), 2);
    assert_eq!(furigana[0]["text"], "食");
    assert_eq!(furigana[0]["reading"], "た");
    assert_eq!(furigana[1]["text"], "べて");
    assert!(furigana[1].get("reading").is_none());

    assert_entry_integrity(&value);
}

#[test]
fn entries_carry_forms_commonness_senses_and_headword_furigana() {
    let value = render_value("食べている");
    let entries = value["entries"].as_object().unwrap();
    assert_eq!(
        entries.keys().collect::<Vec<_>>(),
        ["1358280", "1577980"]
    );

    // 食べる: forms with JMdict priority data (ichiran_latest kanji_text /
    // kana_text rows) and headword ruby on the kanji form.
    let taberu = &entries["1358280"];
    let kanji = taberu["kanji"].as_array().unwrap();
    assert_eq!(kanji[0]["text"], "食べる");
    assert_eq!(kanji[0]["common"], 25);
    assert_eq!(
        kanji[0]["tags"].as_array().unwrap().to_vec(),
        [Value::from("ichi1"), Value::from("news2"), Value::from("nf25")]
    );
    assert_eq!(kanji[0]["furigana"][0]["text"], "食");
    assert_eq!(kanji[0]["furigana"][0]["reading"], "た");
    assert_eq!(kanji[0]["furigana"][1]["text"], "べる");
    assert_eq!(kanji[1]["text"], "喰べる");
    assert!(kanji[1].get("common").is_none());
    assert_eq!(taberu["kana"][0]["text"], "たべる");
    assert_eq!(taberu["kana"][0]["common"], 25);

    let senses = taberu["senses"].as_array().unwrap();
    assert_eq!(
        senses[0]["pos"].as_array().unwrap().to_vec(),
        [Value::from("v1"), Value::from("vt")]
    );
    assert_eq!(
        senses[0]["gloss"].as_array().unwrap().to_vec(),
        [Value::from("to eat")]
    );

    // 居る: kanji form not marked common while the kana form is — plus
    // every sense tagged uk ("usually kana") — the popup's cue to display いる.
    let iru = &entries["1577980"];
    assert!(iru["kanji"][0].get("common").is_none());
    assert_eq!(iru["kana"][0]["text"], "いる");
    assert_eq!(iru["kana"][0]["common"], 0);
    assert_eq!(
        iru["kana"][0]["tags"].as_array().unwrap().to_vec(),
        [Value::from("ichi1")]
    );
    let iru_senses = iru["senses"].as_array().unwrap();
    assert!(!iru_senses.is_empty());
    for sense in iru_senses {
        assert_eq!(
            sense["misc"].as_array().unwrap().to_vec(),
            [Value::from("uk")],
            "every 居る sense is tagged uk"
        );
    }
}

#[test]
fn reading_restricted_senses_expose_their_restrictions() {
    // 頭 (1582310) has senses valid only for あたま and one only for かしら;
    // entries are shared per entry, so the senses say so instead of being
    // pre-filtered (ichiran_latest sense_prop stagr rows). Bare 頭 resolves
    // to the とう counter entry (1450690), so give it a clause.
    let value = render_value("頭を洗う");
    let token = &value["tokens"][0];
    assert_eq!(token["entry"], 1582310);
    assert_eq!(token["reading"], "あたま");

    let senses = value["entries"]["1582310"]["senses"].as_array().unwrap();
    let restricted_to = |reading: &str| {
        senses
            .iter()
            .filter(|sense| {
                sense["restrict_kana"]
                    .as_array()
                    .is_some_and(|restrict| restrict.iter().any(|value| value == reading))
            })
            .count()
    };
    assert_eq!(restricted_to("あたま"), 4);
    assert_eq!(restricted_to("かしら"), 1);
}

#[test]
fn tied_alternatives_are_slim_refs_with_their_own_scores() {
    // がくせい ties 学生 (1206900) and 学制 (1761180) at one span. The token
    // holds the winner; the tie is an entry reference with its own score —
    // not the winner's (the v1 scores are 240 vs 176).
    let value = render_value("わたしはがくせいです");
    assert_eq!(token_texts(&value), ["わたし", "は", "がくせい", "です"]);

    let gakusei = &value["tokens"][2];
    assert_eq!(gakusei["entry"], 1206900);
    assert_eq!(gakusei["score"], 240);
    assert!(gakusei.get("compound").is_none());
    let alternatives = gakusei["alternatives"].as_array().unwrap();
    assert_eq!(alternatives.len(), 1);
    assert_eq!(alternatives[0]["entry"], 1761180);
    assert_eq!(alternatives[0]["score"], 176);
    assert_eq!(alternatives[0]["reading"], "がくせい");
    assert_entry_integrity(&value);

    // Kanji homograph ties collapse the same way: 一日 = いちにち (1576260)
    // with ついたち (2225040) as the alternative.
    let value = render_value("一日");
    let tokens = value["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0]["entry"], 1576260);
    let alternatives = tokens[0]["alternatives"].as_array().unwrap();
    assert_eq!(alternatives.len(), 1);
    assert_eq!(alternatives[0]["entry"], 2225040);
    assert_eq!(alternatives[0]["reading"], "ついたち");
    assert_entry_integrity(&value);
}

#[test]
fn include_entries_false_drops_only_the_entries_table() {
    // The tokens stay at full detail — furigana included.
    let value = render_value_with(
        "食べている",
        V2Options {
            include_entries: false,
            ..V2Options::default()
        },
    );
    assert!(value.get("entries").is_none());
    let tokens = value["tokens"].as_array().unwrap();
    assert_eq!(tokens[0]["furigana"][0]["text"], "食");
    assert_eq!(tokens[0]["conjugation"][0]["base_form"], "食べる");
}

#[test]
fn include_furigana_false_drops_ruby_everywhere_else_stays() {
    let value = render_value_with(
        "食べている",
        V2Options {
            include_furigana: false,
            ..V2Options::default()
        },
    );
    let tokens = value["tokens"].as_array().unwrap();
    assert!(tokens[0].get("furigana").is_none());
    // Analyses, suffix, and the entries table are untouched — including
    // kanji forms, just without their ruby.
    assert_eq!(tokens[0]["conjugation"][0]["base_form"], "食べる");
    assert_eq!(tokens[1]["suffix"]["class"], "iru");
    let taberu_kanji = &value["entries"]["1358280"]["kanji"][0];
    assert_eq!(taberu_kanji["text"], "食べる");
    assert!(taberu_kanji.get("furigana").is_none());
}
