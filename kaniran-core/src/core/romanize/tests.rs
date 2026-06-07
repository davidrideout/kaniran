use super::*;
use crate::characters::kani_kana_class::KanaClass;
use crate::core::methods::{
    hepburn_simple, hepburn_traditional, GenericHepburn, KunreiSiki, RomanizationMethod,
    SimplifiedHepburn, TraditionalHepburn,
};
use crate::core::romanize::get_character_classes;
use crate::dict::split::segsplit::KANA_HINT_MOD;
use crate::dict::split::segsplit::KANA_HINT_SPACE;
use crate::dict::word_info::WordInfoType;

// --- get_character_classes ---
fn class(kana: KanaClass) -> CcItem {
    CcItem::Class(kana)
}

#[test]
fn get_character_classes_fixtures() {
    use KanaClass::*;
    let cases: Vec<(&str, Vec<CcItem>)> = vec![
        ("し", vec![class(Shi)]),
        ("による", vec![class(Ni), class(Yo), class(Ru)]),
        // long-vowel modifier
        (
            "コーヒー",
            vec![class(Ko), class(LongVowel), class(Hi), class(LongVowel)],
        ),
        // sokuon
        ("きっぷ", vec![class(Ki), class(Sokuon), class(Pu)]),
        // iteration marks
        ("ゝゞ", vec![class(Iter), class(IterV)]),
        // non-kana glyphs return the char itself
        (
            "Aと5",
            vec![CcItem::Char('A'), class(To), CcItem::Char('5')],
        ),
        // kanji all fall back to chars
        ("東京", vec![CcItem::Char('東'), CcItem::Char('京')]),
    ];
    for (word, expected) in &cases {
        assert_eq!(&get_character_classes(word), expected, "word={word:?}");
    }
}

// --- leftmost_atom ---
fn atom(kana: KanaClass) -> CcTree {
    CcTree::Atom(CcItem::Class(kana))
}
fn node(kana: KanaClass, tail: Vec<CcTree>) -> CcTree {
    CcTree::Node(kana, tail)
}

#[test]
fn leftmost_atom_fixtures() {
    use KanaClass::*;
    // Each row is (label, input cc-tree, expected leftmost atom).
    let cases: Vec<(&str, Vec<CcTree>, Option<CcItem>)> = vec![
        // first element is already an atom
        ("(:TA)", vec![atom(Ta)], Some(CcItem::Class(Ta))),
        // flat list returns the head
        (
            "(:SO :U :SHI)",
            vec![atom(So), atom(U), atom(Shi)],
            Some(CcItem::Class(So)),
        ),
        // descends into a modifier node
        (
            "((:+YA :CHI))",
            vec![node(PlusYa, vec![atom(Chi)])],
            Some(CcItem::Class(Chi)),
        ),
        // descends through a sokuon node
        (
            "((:SOKUON (:+YA :CHI)))",
            vec![node(Sokuon, vec![node(PlusYa, vec![atom(Chi)])])],
            Some(CcItem::Class(Chi)),
        ),
        // descends through nested modifiers
        (
            "((:+YU (:+YA :CHI)))",
            vec![node(PlusYu, vec![node(PlusYa, vec![atom(Chi)])])],
            Some(CcItem::Class(Chi)),
        ),
        // empty list is nil
        ("NIL", vec![], None),
        // a nil leaf is the leftmost atom
        ("((:+YA NIL))", vec![node(PlusYa, vec![CcTree::Nil])], None),
        // a char leaf
        (
            "(#\\a)",
            vec![CcTree::Atom(CcItem::Char('a'))],
            Some(CcItem::Char('a')),
        ),
    ];
    for (label, input, expected) in &cases {
        assert_eq!(&leftmost_atom(input), expected, "case={label}");
    }
}

// --- romanize_core ---
#[test]
fn romanize_core_walks_every_node_shape() {
    use KanaClass::*;
    let hepburn = GenericHepburn::new();
    let method = RomanizationMethod::GenericHepburn(&hepburn);
    // One tree exercising a mora class, a nil skip, raw character
    // passthrough, and a modifier node.
    let cc_tree = vec![
        CcTree::Atom(CcItem::Class(Ko)),
        CcTree::Nil,
        CcTree::Atom(CcItem::Class(N)),
        CcTree::Atom(CcItem::Char('x')),
        CcTree::Node(PlusYa, vec![CcTree::Atom(CcItem::Class(Ki))]),
    ];
    assert_eq!(romanize_core(method, &cc_tree), "kon'xkya");
}

// --- romanize_list ---
#[test]
fn romanize_list_fixtures() {
    let generic = GenericHepburn::new();
    let simple = SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]);
    let traditional = TraditionalHepburn::new();
    let kunrei = KunreiSiki::new();
    let basic = RomanizationMethod::GenericHepburn(&generic);
    let simple = RomanizationMethod::SimplifiedHepburn(&simple);
    let traditional = RomanizationMethod::TraditionalHepburn(&traditional);
    let kunrei = RomanizationMethod::KunreiSiki(&kunrei);
    // (word, method, expected).
    let cases: &[(&str, RomanizationMethod, &str)] = &[
        ("きっぷ", traditional, "kippu"),
        ("きっぷ", kunrei, "kippu"),
        ("まっちゃ", traditional, "matcha"),
        ("まっちゃ", kunrei, "mattya"),
        ("コーヒー", traditional, "kohi"),
        ("しゃしん", basic, "shashin"),
        ("しゃしん", kunrei, "syasin"),
        ("がっこう", traditional, "gakkō"),
        ("がっこう", basic, "gakkou"),
        ("がっこう", kunrei, "gakkô"),
        ("がっこう", simple, "gakko"),
        ("こんにちは", traditional, "konnichiha"),
        ("こんにちは", kunrei, "konnitiha"),
        ("とうきょう", traditional, "tōkyō"),
        ("とうきょう", basic, "toukyou"),
        ("とうきょう", kunrei, "tôkyô"),
        ("とうきょう", simple, "tokyo"),
        ("ありがとう", traditional, "arigatō"),
        ("しんぶん", traditional, "shimbun"),
        ("しんぶん", basic, "shinbun"),
        ("しんぶん", kunrei, "sinbun"),
    ];
    for (word, method, expected) in cases {
        let cc_list = get_character_classes(word);
        assert_eq!(&romanize_list(&cc_list, *method), expected, "word={word}");
    }
}

// --- romanize_word ---
#[test]
fn romanize_word_fixtures() {
    let traditional = RomanizationMethod::TraditionalHepburn(hepburn_traditional());

    // A lone small tsu / long-vowel bar romanize to "!" / "~" regardless
    // of method.
    assert_eq!(romanize_word("っ", traditional, None, true), "!");
    assert_eq!(romanize_word("ー", traditional, None, true), "~");

    // normalize=false, with an original spelling supplied.
    assert_eq!(
        romanize_word("センター", traditional, Some("センター"), false),
        "senta"
    );
    assert_eq!(romanize_word("よ", traditional, Some("予"), false), "yo");

    // normalize=true: half-width katakana folds to full-width, then romanizes.
    assert_eq!(romanize_word("ｾﾝﾀｰ", traditional, None, true), "senta");
    assert_eq!(
        romanize_word("こんにちは", traditional, None, true),
        "konnichiha"
    );

    // When the original spelling is a lone small tsu / long-vowel bar, it
    // romanizes to "!" / "~" and the word argument is ignored.
    assert_eq!(romanize_word("x", traditional, Some("っ"), false), "!");
    assert_eq!(romanize_word("x", traditional, Some("ー"), false), "~");

    // An empty original spelling is not a special glyph, so the word is
    // romanized normally.
    assert_eq!(romanize_word("よむ", traditional, Some(""), false), "yomu");
    assert_eq!(
        romanize_word("しゃしん", traditional, Some(""), false),
        "shashin"
    );

    // Method variation: kunrei-siki spells し differently from hepburn.
    let kunrei_inst = KunreiSiki::new();
    let kunrei = RomanizationMethod::KunreiSiki(&kunrei_inst);
    assert_eq!(romanize_word("しゃしん", kunrei, None, true), "syasin");
}

#[test]
fn romanize_word_process_hints() {
    // Hint processing acts on the word, not on the original spelling.
    let traditional = RomanizationMethod::TraditionalHepburn(hepburn_traditional());

    // A modifier-hint sentinel before は turns it into "wa".
    let hint_ha = format!("{}は", KANA_HINT_MOD);
    assert_eq!(romanize_word(&hint_ha, traditional, None, false), "wa");
    // An original spelling of "は" does not block the hint.
    assert_eq!(
        romanize_word(&hint_ha, traditional, Some("は"), false),
        "wa"
    );

    // A space-hint sentinel before へ produces a leading space: " he".
    let hint_space = format!("{}へ", KANA_HINT_SPACE);
    assert_eq!(romanize_word(&hint_space, traditional, None, false), " he");
}

// --- romanize_word_geo ---
#[test]
fn romanize_word_geo_fixtures() {
    // Place names, using the default simplified-hepburn method.
    let simple = RomanizationMethod::SimplifiedHepburn(hepburn_simple());
    let cases: &[(&str, &str)] = &[
        ("とうきょう", "Tokyo"),
        ("おおさか", "Osaka"),
        ("ほっかいどう", "Hokkaido"),
        ("ぐんま", "Gunma"),
        // ん before お inserts an apostrophe boundary, and the vowel after
        // it is capitalized: "Shin'Osaka".
        ("しんおおさか", "Shin'Osaka"),
        ("きょうと", "Kyoto"),
        ("ふじさん", "Fujisan"),
        ("しんじゅく", "Shinjuku"),
        // a lone small tsu / long-vowel bar romanize to "!" / "~".
        ("っ", "!"),
        ("ー", "~"),
        // empty input
        ("", ""),
        // half-width katakana normalizes to full width before romanizing
        ("ﾄｳｷｮｳ", "Tokyo"),
        // kanji is not in the kana table, so it passes through unchanged
        ("東京", "東京"),
        ("ニューヨーク", "Nyuyoku"),
    ];
    for (input, expected) in cases {
        assert_eq!(&romanize_word_geo(input, simple), expected, "input={input}");
    }
}

#[test]
fn romanize_word_geo_method_param() {
    // A method argument overrides the simplified-hepburn default, here
    // producing macron long vowels.
    let traditional = RomanizationMethod::TraditionalHepburn(hepburn_traditional());
    assert_eq!(romanize_word_geo("とうきょう", traditional), "Tōkyō");
    assert_eq!(romanize_word_geo("おおさか", traditional), "Ōsaka");
}

#[test]
fn string_capitalize_fixtures() {
    let cases: &[(&str, &str)] = &[
        // apostrophe word boundary
        ("shin'osaka", "Shin'Osaka"),
        ("n'pou", "N'Pou"),
        // space-delimited words; trailing letters downcased
        ("hello world", "Hello World"),
        ("ABC DEF", "Abc Def"),
        // interior digits do not break the word
        ("abc123def", "Abc123def"),
        ("a5b", "A5b"),
        // leading digit is alphanumeric but uncased; the run stays one word
        ("5abc", "5abc"),
        // hyphen is not alphanumeric, so it starts a new word
        ("foo-bar", "Foo-Bar"),
        // non-alphanumeric-only and empty inputs
        ("!", "!"),
        ("~", "~"),
        ("", ""),
        // ideographs are alphanumeric (Lo) but have no case change
        ("東京", "東京"),
    ];
    for (input, expected) in cases {
        assert_eq!(&string_capitalize(input), expected, "input={input}");
    }
}

// --- join_parts ---
#[test]
fn join_parts_fixtures() {
    let cases: &[(&[&str], &str)] = &[
        // spaces inserted between alphanumeric parts
        (
            &["watashi", "wa", "gakusei", "desu"],
            "watashi wa gakusei desu",
        ),
        // no space before punctuation
        (&["Tokyo", ",", "desu"], "Tokyo, desu"),
        // a trailing space suppresses the next part's space
        (&["hello ", "world"], "hello world"),
        // leading empty part: last_space stays true, no leading space
        (&["", "abc"], "abc"),
        // empty middle part leaves last_space false, so "def" still spaces
        (&["abc", "", "def"], "abc def"),
        // ideographic period is not alphanumeric
        (&["Tokyo", "。"], "Tokyo。"),
        // ① (circled number) is not treated as alphanumeric, so no space
        (&["a", "①"], "a①"),
        // Ⅴ (roman numeral) is not alphanumeric; the space before "a"
        // comes from "a", not Ⅴ
        (&["Ⅴ", "a"], "Ⅴ a"),
        // ascii digit is alphanumeric
        (&["a", "5"], "a 5"),
        // fullwidth ５ (category Nd) is alphanumeric
        (&["a", "５"], "a ５"),
        // prolonged sound mark ー (category Lm) is alphanumeric
        (&["a", "ー"], "a ー"),
        // trailing U+3000 ideographic space sets the whitespace flag
        (&["foo　", "bar"], "foo　bar"),
    ];
    for (parts, expected) in cases {
        assert_eq!(&join_parts(parts), expected, "parts={parts:?}");
    }
}

// --- romanize_word_info ---
fn wi(text: &str, kana: Option<WordInfoKana>) -> WordInfo {
    WordInfo {
        kind: WordInfoType::Kana,
        text: text.to_string(),
        kana,
        ..Default::default()
    }
}

fn single(reading: &str) -> Option<WordInfoKana> {
    Some(WordInfoKana::Single(reading.to_string()))
}

#[test]
fn romanize_word_info_fixtures() {
    let traditional =
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
    let kana = KaniRomanizeMethod::Kana;

    // 教会 — single-string kana, long-vowel macrons under traditional.
    let kyoukai = wi("教会", single("きょうかい"));
    assert_eq!(romanize_word_info(&kyoukai, traditional), "kyōkai");
    assert_eq!(romanize_word_info(&kyoukai, kana), "きょうかい");

    // 小学校 — geminate + macron.
    let shougakkou = wi("小学校", single("しょうがっこう"));
    assert_eq!(romanize_word_info(&shougakkou, traditional), "shōgakkō");
    assert_eq!(romanize_word_info(&shougakkou, kana), "しょうがっこう");

    // は — the kana carries a modifier-hint sentinel: a method romanizes
    // it to "wa", while :kana strips the sentinel back to "は".
    let hinted_ha = wi("は", single(&format!("{}は", KANA_HINT_MOD)));
    assert_eq!(romanize_word_info(&hinted_ha, traditional), "wa");
    assert_eq!(romanize_word_info(&hinted_ha, kana), "は");

    // はた — a kana list with a single reading.
    let hata = wi("はた", Some(WordInfoKana::Multi(vec![single("はた")])));
    assert_eq!(romanize_word_info(&hata, traditional), "hata");
    assert_eq!(romanize_word_info(&hata, kana), "はた");

    // A lone small tsu / long-vowel bar romanizes to "!" / "~" under a
    // method, and stays itself under :kana.
    let tsu = wi("っ", single("っ"));
    assert_eq!(romanize_word_info(&tsu, traditional), "!");
    assert_eq!(romanize_word_info(&tsu, kana), "っ");
    let bar = wi("ー", single("ー"));
    assert_eq!(romanize_word_info(&bar, traditional), "~");
    assert_eq!(romanize_word_info(&bar, kana), "ー");
}

#[test]
fn romanize_word_info_method_arm_nil_element() {
    // Under a method, an empty element in a kana list romanizes to "",
    // so kana=("あ", empty) gives "a/".
    let traditional =
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
    let wi_nil_elem = wi("x", Some(WordInfoKana::Multi(vec![single("あ"), None])));
    assert_eq!(romanize_word_info(&wi_nil_elem, traditional), "a/");
}

#[test]
#[should_panic]
fn romanize_word_info_kana_arm_nil_element_errors() {
    // The :kana path errors on an empty element in a kana list.
    let wi_nil_elem = wi("x", Some(WordInfoKana::Multi(vec![single("あ"), None])));
    romanize_word_info(&wi_nil_elem, KaniRomanizeMethod::Kana);
}

#[test]
#[should_panic]
fn romanize_word_info_nested_element_errors() {
    // A nested-list kana element is an error under a method.
    let traditional =
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
    let wi_nested = wi(
        "x",
        Some(WordInfoKana::Multi(vec![Some(WordInfoKana::Multi(vec![
            single("あ"),
        ]))])),
    );
    romanize_word_info(&wi_nested, traditional);
}

#[test]
fn romanize_word_info_nil_kana() {
    // A word-info with no kana yields "".
    let traditional =
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
    let empty = wi("x", None);
    assert_eq!(romanize_word_info(&empty, traditional), "");
    assert_eq!(romanize_word_info(&empty, KaniRomanizeMethod::Kana), "");
}

// --- romanize ---
// These tests hit the database; run with `-- --test-threads=1`.

async fn romanize_ctx() -> std::sync::Arc<KaniranContext> {
    KaniranContext::from_env()
        .await
        .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
}

fn romanize_traditional() -> KaniRomanizeMethod<'static> {
    KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
}

#[tokio::test]
async fn romanize_joined_string() {
    // Each row: (input, default-method joined string, :kana joined string).
    // The default method converts 。/！ to ". "/"! "; the :kana output
    // keeps the full-width punctuation.
    let romanize_ctx = romanize_ctx().await;
    let cases: &[(&str, &str, &str)] = &[
        (
            "富士山は日本で最も高い山である。",
            "fujisan wa nihon de mottomo takai yama de aru. ",
            "ふじさん は にほん で もっとも たかい やま で ある。",
        ),
        (
            "2020年に東京オリンピックが開催された。",
            "nisen nijūnen ni tōkyō orimpikku ga kaisai sareta. ",
            "にせんにじゅうねん に とうきょう オリンピック が かいさい された。",
        ),
        (
            "彼女は新しい仮説を提唱した。",
            "kanojo wa atarashii kasetsu wo teishō shita. ",
            "かのじょ は あたらしい かせつ を ていしょう した。",
        ),
        (
            "ABCは試験的な略語です。",
            "ABC wa shikenteki na ryakugo desu. ",
            "ABC は しけんてき な りゃくご です。",
        ),
        ("Hello 世界！", "Hello sekai! ", "Hello せかい！"),
    ];
    for (input, expected_default, expected_kana) in cases {
        let (default_str, _) = romanize(&romanize_ctx, input, romanize_traditional(), false)
            .await
            .unwrap();
        assert_eq!(
            &default_str, expected_default,
            "default method, input={input:?}"
        );
        let (kana_str, _) = romanize(&romanize_ctx, input, KaniRomanizeMethod::Kana, false)
            .await
            .unwrap();
        assert_eq!(&kana_str, expected_kana, ":kana method, input={input:?}");
    }
}

#[tokio::test]
async fn romanize_with_info_collects_definitions_in_order() {
    // With info on, each romanized part gets one definition paired with its
    // headword; the trailing ". " punctuation contributes none.
    let romanize_ctx = romanize_ctx().await;
    let (joined, definitions) = romanize(
        &romanize_ctx,
        "富士山は日本で最も高い山である。",
        romanize_traditional(),
        true,
    )
    .await
    .unwrap();
    assert_eq!(joined, "fujisan wa nihon de mottomo takai yama de aru. ");
    let roms: Vec<&str> = definitions.iter().map(|(rom, _)| rom.as_str()).collect();
    assert_eq!(
        roms,
        vec!["fujisan", "wa", "nihon", "de", "mottomo", "takai", "yama", "de", "aru"]
    );
    // Headwords pair with the romanization in encounter order.
    assert!(
        definitions[0].1.starts_with("富士山"),
        "got {:?}",
        definitions[0].1
    );
    assert!(
        definitions[2].1.starts_with("日本"),
        "got {:?}",
        definitions[2].1
    );
    assert!(
        definitions[6].1.starts_with("山"),
        "got {:?}",
        definitions[6].1
    );
}

#[tokio::test]
async fn romanize_with_info_false_yields_empty_definitions() {
    // with-info nil: the joined string is unchanged and no definitions
    // are collected.
    let romanize_ctx = romanize_ctx().await;
    let (joined, definitions) = romanize(
        &romanize_ctx,
        "彼女は新しい仮説を提唱した。",
        romanize_traditional(),
        false,
    )
    .await
    .unwrap();
    assert_eq!(joined, "kanojo wa atarashii kasetsu wo teishō shita. ");
    assert!(definitions.is_empty());
}

// --- romanize_star_ ---
// These tests hit the database; run with `-- --test-threads=1`.

async fn romanize_star_ctx() -> std::sync::Arc<KaniranContext> {
    KaniranContext::from_env()
        .await
        .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
}

fn romanize_star_traditional() -> KaniRomanizeMethod<'static> {
    KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
}

/// Projection of the result with `prop = ()` dropped: `:misc` splits map
/// to their text, `:word` splits to `(score, [(romanized, word.text)])`
/// per alternative — the shape this function owns.
#[derive(Debug, PartialEq)]
enum SegShape {
    Misc(String),
    Word(Vec<(i32, Vec<(String, String)>)>),
}

fn shape(result: &[RomanizeStarSegment<()>]) -> Vec<SegShape> {
    result
        .iter()
        .map(|segment| match segment {
            RomanizeStarSegment::Misc(text) => SegShape::Misc(text.clone()),
            RomanizeStarSegment::Word(alternatives) => SegShape::Word(
                alternatives
                    .iter()
                    .map(|(word_props, score)| {
                        (
                            *score,
                            word_props
                                .iter()
                                .map(|(rom, word, _)| (rom.clone(), word.text.clone()))
                                .collect(),
                        )
                    })
                    .collect(),
            ),
        })
        .collect()
}

#[tokio::test]
async fn romanize_star_full_structure() {
    // "Hello 世界！" splits into a latin misc, a word with 5 distinct-score
    // alternatives, and a "! " misc. Alternatives carry unromanizable kanji
    // verbatim (e.g. "世" / "世界") when no reading wins.
    let romanize_star_ctx = romanize_star_ctx().await;
    let result = romanize_star_(
        &romanize_star_ctx,
        "Hello 世界！",
        romanize_star_traditional(),
        None,
        |_, _| (),
    )
    .await
    .unwrap();
    let word = |rom: &str, text: &str| (rom.to_string(), text.to_string());
    assert_eq!(
        shape(&result),
        vec![
            SegShape::Misc("Hello ".to_string()),
            SegShape::Word(vec![
                (325, vec![word("sekai", "世界")]),
                (23, vec![word("yo", "世"), word("kai", "界")]),
                (-487, vec![word("世", "世"), word("kai", "界")]),
                (-490, vec![word("yo", "世"), word("界", "界")]),
                (-1000, vec![word("世界", "世界")]),
            ]),
            SegShape::Misc("! ".to_string()),
        ]
    );
}

#[tokio::test]
async fn romanize_star_misc_in_middle() {
    // "ABCは試験的な略語です。" splits into a leading latin misc, a word,
    // and a trailing ". " misc. Checks the segment kinds and the top
    // alternative's word sequence (top score 1091 is unique).
    let romanize_star_ctx = romanize_star_ctx().await;
    let result = romanize_star_(
        &romanize_star_ctx,
        "ABCは試験的な略語です。",
        romanize_star_traditional(),
        None,
        |_, _| (),
    )
    .await
    .unwrap();
    let shaped = shape(&result);
    assert!(matches!(shaped[0], SegShape::Misc(ref t) if t == "ABC"));
    assert!(matches!(shaped[2], SegShape::Misc(ref t) if t == ". "));
    let SegShape::Word(ref alternatives) = shaped[1] else {
        panic!("segment 1 should be a word split, got {:?}", shaped[1]);
    };
    assert_eq!(alternatives.len(), 5);
    let (top_score, ref top_words) = alternatives[0];
    assert_eq!(top_score, 1091);
    let roms: Vec<&str> = top_words.iter().map(|(rom, _)| rom.as_str()).collect();
    assert_eq!(roms, vec!["wa", "shikenteki", "na", "ryakugo", "desu"]);
}

#[tokio::test]
async fn romanize_star_wordprop_fn_receives_romanized_and_word() {
    // The prop callback is given both the romanization and the word; its
    // result becomes the prop in each triple. Here it returns
    // (romanized-byte-len, word text) so both arguments are observed:
    // "sekai" (5 bytes) over word "世界".
    let romanize_star_ctx = romanize_star_ctx().await;
    let result = romanize_star_(
        &romanize_star_ctx,
        "世界",
        romanize_star_traditional(),
        Some(1),
        |rom, word| (rom.len(), word.text.clone()),
    )
    .await
    .unwrap();
    let RomanizeStarSegment::Word(ref alternatives) = result[0] else {
        panic!("expected a word split, got {:?}", result[0]);
    };
    let (ref word_props, score) = alternatives[0];
    assert_eq!(score, 325);
    let (ref rom, ref word, ref prop) = word_props[0];
    assert_eq!(rom, "sekai");
    assert_eq!(word.text, "世界");
    assert_eq!(*prop, (5, "世界".to_string()));
}
