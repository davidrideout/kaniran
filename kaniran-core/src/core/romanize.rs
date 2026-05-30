//! Romanization entry points. From `romanize.lisp:205-290`.

use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

use super::methods::{CcItem, KaniRomanizeMethod, RomanizationMethod};
use super::rules::{
    get_character_classes, join_parts, process_iteration_characters, process_modifiers,
    r_simplify, r_special, romanize_core,
};
use crate::characters::normalize::{normalize as char_normalize, NormalizationContext};
use crate::characters::text_utils::{basic_split, SegmentKind};
use crate::conn::kani_context::KaniranContext;
use crate::dict::dict_segment::dict_segment;
use crate::dict::map_word_info_kana::map_word_info_kana;
use crate::dict::process_hints::process_hints;
use crate::dict::simple_segment::simple_segment;
use crate::dict::strip_hints::strip_hints;
use crate::dict::word_info_class::{WordInfo, WordInfoKana};
use crate::dict::word_info_str::word_info_str;

/// `romanize-list` (`romanize.lisp:205-208`). Expand iteration markers,
/// fold modifiers into a tree, romanize it, then simplify per the
/// method.
pub fn romanize_list(cc_list: &[CcItem], method: RomanizationMethod<'_>) -> String {
    let cc_tree = process_modifiers(&process_iteration_characters(cc_list));
    r_simplify(method, &romanize_core(method, &cc_tree))
}

/// `romanize-word` (`romanize.lisp:224-230`). Normalize `word` if
/// requested, check `r-special` against `original_spelling` (or the
/// word), and otherwise process hints and run `romanize-list` over the
/// character-class sequence. Empty `original_spelling = Some("")` is
/// truthy in CL — `r-special("")` is nil, so the word is romanized
/// normally.
pub fn romanize_word(
    word: &str,
    method: RomanizationMethod<'_>,
    original_spelling: Option<&str>,
    normalize: bool,
) -> String {
    let normalized;
    let word: &str = if normalize {
        normalized = char_normalize(word, NormalizationContext::Default);
        &normalized
    } else {
        word
    };
    if let Some(special) = r_special(method, original_spelling.unwrap_or(word)) {
        return special;
    }
    let word = process_hints(word);
    romanize_list(&get_character_classes(&word), method)
}

/// `romanize-word-geo` (`romanize.lisp:232-233`). `string-capitalize`
/// of `romanize-word`. Upstream default method is `*hepburn-simple*`.
pub fn romanize_word_geo(input: &str, method: RomanizationMethod<'_>) -> String {
    string_capitalize(&romanize_word(input, method, None, true))
}

/// `string-capitalize` (CL builtin): upcase the first cased character
/// of every alphanumeric-delimited word and downcase the rest; a
/// non-alphanumeric character passes through and starts a new word.
fn string_capitalize(string: &str) -> String {
    let mut out = String::with_capacity(string.len());
    let mut newword = true;
    for char in string.chars() {
        if !alphanumericp(char) {
            out.push(char);
            newword = true;
        } else if newword {
            out.extend(char.to_uppercase());
            newword = false;
        } else {
            out.extend(char.to_lowercase());
        }
    }
    out
}

fn alphanumericp(char: char) -> bool {
    char.general_category_group() == GeneralCategoryGroup::Letter
        || char.general_category() == GeneralCategory::DecimalNumber
}

/// `romanize-word-info` (`romanize.lisp:248-255`). Per-`wk`
/// romanization through `map-word-info-kana`. Under `:kana`, `wk` is
/// stripped of hints; under a method, `wk` is romanized with
/// `original-spelling = word.text`, `normalize = nil`.
pub fn romanize_word_info(word_info: &WordInfo, method: KaniRomanizeMethod<'_>) -> String {
    let orig_text = &word_info.text;
    match method {
        KaniRomanizeMethod::Kana => {
            map_word_info_kana(|wk| strip_hints(strip_hints_input(wk)), word_info, "/")
        }
        KaniRomanizeMethod::Method(method) => map_word_info_kana(
            |wk| romanize_word(romanize_word_input(wk), method, Some(orig_text), false),
            word_info,
            "/",
        ),
    }
}

/// The kana element `map-word-info-kana` binds to `wk` for the
/// romanize-word closure. Every element a segmenter word-info carries
/// is a reading string (corpus: 928764/928764 Single). A nil element
/// romanizes to "" upstream; a nested list panics (upstream type
/// error in `romanize-word`'s `process-hints`).
fn romanize_word_input(wk: &Option<WordInfoKana>) -> &str {
    match wk {
        Some(WordInfoKana::Single(reading)) => reading,
        None => "",
        Some(WordInfoKana::Multi(_)) => {
            panic!("romanize-word-info: nested-list kana element (upstream romanize-word type error)")
        }
    }
}

/// As [`romanize_word_input`] but for the `:kana` (strip-hints)
/// closure. A nil element errors upstream rather than yielding "";
/// nested-list errors the same way. Only reading strings occur in
/// practice.
fn strip_hints_input(wk: &Option<WordInfoKana>) -> &str {
    match wk {
        Some(WordInfoKana::Single(reading)) => reading,
        _ => panic!(
            "romanize-word-info :kana: non-string kana element (upstream simplify-reading-list error)"
        ),
    }
}

/// `romanize` (`romanize.lisp:257-271`). Normalize, basic-split, then
/// for each word split run `simple-segment` and `romanize-word-info`.
/// `with_info = true` collects `(rom, word-info-str)` pairs in encounter
/// order. Returns `(joined-string, definitions)`.
pub async fn romanize(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    with_info: bool,
) -> Result<(String, Vec<(String, String)>), sqlx::Error> {
    let context = match method {
        KaniRomanizeMethod::Kana => NormalizationContext::Kana,
        KaniRomanizeMethod::Method(_) => NormalizationContext::Default,
    };
    let input = char_normalize(input, context);
    let mut definitions: Vec<(String, String)> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    for (split_type, split_text) in basic_split(&input) {
        if split_type == SegmentKind::Word {
            for word in simple_segment(ctx, &split_text, None).await? {
                let rom = romanize_word_info(&word, method);
                if with_info {
                    definitions.push((rom.clone(), word_info_str(ctx, &word).await?));
                }
                parts.push(rom);
            }
        } else {
            parts.push(split_text);
        }
    }
    Ok((join_parts(&parts), definitions))
}

/// `romanize*` (`romanize.lisp:273-290`). Each `basic-split` segment
/// becomes a [`RomanizeStarSegment`]: `:misc` → `Misc(text)`, `:word`
/// → `Word` holding one `(word-prop-list, score)` pair per
/// `dict-segment` alternative. The Lisp `(list romanized word prop)`
/// triple becomes a `(String, WordInfo, P)` tuple.
pub async fn romanize_star_<P>(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    limit: Option<usize>,
    wordprop_fn: impl Fn(&str, &WordInfo) -> P,
) -> Result<Vec<RomanizeStarSegment<P>>, sqlx::Error> {
    let context = match method {
        KaniRomanizeMethod::Kana => NormalizationContext::Kana,
        KaniRomanizeMethod::Method(_) => NormalizationContext::Default,
    };
    let input = char_normalize(input, context);
    let mut result: Vec<RomanizeStarSegment<P>> = Vec::new();
    for (split_type, split_text) in basic_split(&input) {
        if split_type == SegmentKind::Word {
            let mut alternatives = Vec::new();
            for (word_list, score) in dict_segment(ctx, &split_text, limit).await? {
                let mut word_props = Vec::new();
                for word in word_list {
                    let romanized = romanize_word_info(&word, method);
                    let prop = wordprop_fn(&romanized, &word);
                    word_props.push((romanized, word, prop));
                }
                alternatives.push((word_props, score));
            }
            result.push(RomanizeStarSegment::Word(alternatives));
        } else {
            result.push(RomanizeStarSegment::Misc(split_text));
        }
    }
    Ok(result)
}

#[derive(Debug)]
pub enum RomanizeStarSegment<P> {
    Misc(String),
    Word(Vec<(Vec<(String, WordInfo, P)>, i32)>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::methods::{
        hepburn_simple, hepburn_traditional, GenericHepburn, KunreiSiki, SimplifiedHepburn,
        TraditionalHepburn,
    };
    use crate::dict::_star_kana_hint_mod_star_::KANA_HINT_MOD;
    use crate::dict::_star_kana_hint_space_star_::KANA_HINT_SPACE;
    use crate::dict::word_info_class::WordInfoType;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn traditional() -> KaniRomanizeMethod<'static> {
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
    }

    /// REPL fixtures (.103, (romanize-list (get-character-classes W) :method M)), 2026-05-24.
    #[test]
    fn romanize_list_fixtures() {
        let generic = GenericHepburn::new();
        let simple = SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]);
        let trad = TraditionalHepburn::new();
        let kunrei = KunreiSiki::new();
        let basic = RomanizationMethod::GenericHepburn(&generic);
        let simple = RomanizationMethod::SimplifiedHepburn(&simple);
        let trad = RomanizationMethod::TraditionalHepburn(&trad);
        let kunrei = RomanizationMethod::KunreiSiki(&kunrei);
        let cases: &[(&str, RomanizationMethod, &str)] = &[
            ("きっぷ", trad, "kippu"),
            ("きっぷ", kunrei, "kippu"),
            ("まっちゃ", trad, "matcha"),
            ("まっちゃ", kunrei, "mattya"),
            ("コーヒー", trad, "kohi"),
            ("しゃしん", basic, "shashin"),
            ("しゃしん", kunrei, "syasin"),
            ("がっこう", trad, "gakkō"),
            ("がっこう", basic, "gakkou"),
            ("がっこう", kunrei, "gakkô"),
            ("がっこう", simple, "gakko"),
            ("こんにちは", trad, "konnichiha"),
            ("こんにちは", kunrei, "konnitiha"),
            ("とうきょう", trad, "tōkyō"),
            ("とうきょう", basic, "toukyou"),
            ("とうきょう", kunrei, "tôkyô"),
            ("とうきょう", simple, "tokyo"),
            ("ありがとう", trad, "arigatō"),
            ("しんぶん", trad, "shimbun"),
            ("しんぶん", basic, "shinbun"),
            ("しんぶん", kunrei, "sinbun"),
        ];
        for (word, method, expected) in cases {
            let cc_list = get_character_classes(word);
            assert_eq!(&romanize_list(&cc_list, *method), expected, "word={word}");
        }
    }

    /// REPL fixtures (.103, ichiran:romanize-word), 2026-05-24.
    #[test]
    fn romanize_word_fixtures() {
        let trad = RomanizationMethod::TraditionalHepburn(hepburn_traditional());
        assert_eq!(romanize_word("っ", trad, None, true), "!");
        assert_eq!(romanize_word("ー", trad, None, true), "~");
        assert_eq!(
            romanize_word("センター", trad, Some("センター"), false),
            "senta"
        );
        assert_eq!(romanize_word("よ", trad, Some("予"), false), "yo");
        assert_eq!(romanize_word("ｾﾝﾀｰ", trad, None, true), "senta");
        assert_eq!(romanize_word("こんにちは", trad, None, true), "konnichiha");
        assert_eq!(romanize_word("x", trad, Some("っ"), false), "!");
        assert_eq!(romanize_word("x", trad, Some("ー"), false), "~");
        assert_eq!(romanize_word("よむ", trad, Some(""), false), "yomu");
        assert_eq!(romanize_word("しゃしん", trad, Some(""), false), "shashin");

        let kunrei_inst = KunreiSiki::new();
        let kunrei = RomanizationMethod::KunreiSiki(&kunrei_inst);
        assert_eq!(romanize_word("しゃしん", kunrei, None, true), "syasin");
    }

    /// REPL fixtures (.103, ichiran:romanize-word), 2026-05-24 — the
    /// process-hints branch operates on the (possibly normalized)
    /// word, not on original-spelling.
    #[test]
    fn romanize_word_process_hints() {
        let trad = RomanizationMethod::TraditionalHepburn(hepburn_traditional());

        let hint_ha = format!("{}は", KANA_HINT_MOD);
        assert_eq!(romanize_word(&hint_ha, trad, None, false), "wa");
        assert_eq!(romanize_word(&hint_ha, trad, Some("は"), false), "wa");

        let hint_space = format!("{}へ", KANA_HINT_SPACE);
        assert_eq!(romanize_word(&hint_space, trad, None, false), " he");
    }

    /// REPL fixtures (.103, ichiran:romanize-word-geo), 2026-05-24.
    #[test]
    fn romanize_word_geo_fixtures() {
        let simple = RomanizationMethod::SimplifiedHepburn(hepburn_simple());
        let cases: &[(&str, &str)] = &[
            ("とうきょう", "Tokyo"),
            ("おおさか", "Osaka"),
            ("ほっかいどう", "Hokkaido"),
            ("ぐんま", "Gunma"),
            ("しんおおさか", "Shin'Osaka"),
            ("きょうと", "Kyoto"),
            ("ふじさん", "Fujisan"),
            ("しんじゅく", "Shinjuku"),
            ("っ", "!"),
            ("ー", "~"),
            ("", ""),
            ("ﾄｳｷｮｳ", "Tokyo"),
            ("東京", "東京"),
            ("ニューヨーク", "Nyuyoku"),
        ];
        for (input, expected) in cases {
            assert_eq!(&romanize_word_geo(input, simple), expected, "input={input}");
        }
    }

    #[test]
    fn romanize_word_geo_method_param() {
        let trad = RomanizationMethod::TraditionalHepburn(hepburn_traditional());
        assert_eq!(romanize_word_geo("とうきょう", trad), "Tōkyō");
        assert_eq!(romanize_word_geo("おおさか", trad), "Ōsaka");
    }

    /// REPL fixtures (.103, cl:string-capitalize), 2026-05-24.
    #[test]
    fn string_capitalize_fixtures() {
        let cases: &[(&str, &str)] = &[
            ("shin'osaka", "Shin'Osaka"),
            ("n'pou", "N'Pou"),
            ("hello world", "Hello World"),
            ("ABC DEF", "Abc Def"),
            ("abc123def", "Abc123def"),
            ("a5b", "A5b"),
            ("5abc", "5abc"),
            ("foo-bar", "Foo-Bar"),
            ("!", "!"),
            ("~", "~"),
            ("", ""),
            ("東京", "東京"),
        ];
        for (input, expected) in cases {
            assert_eq!(&string_capitalize(input), expected, "input={input}");
        }
    }

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

    /// REPL fixtures (.103, ichiran:romanize-word-info via simple-segment), 2026-05-24.
    #[test]
    fn romanize_word_info_fixtures() {
        let trad =
            KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
        let kana = KaniRomanizeMethod::Kana;

        let kyoukai = wi("教会", single("きょうかい"));
        assert_eq!(romanize_word_info(&kyoukai, trad), "kyōkai");
        assert_eq!(romanize_word_info(&kyoukai, kana), "きょうかい");

        let shougakkou = wi("小学校", single("しょうがっこう"));
        assert_eq!(romanize_word_info(&shougakkou, trad), "shōgakkō");
        assert_eq!(romanize_word_info(&shougakkou, kana), "しょうがっこう");

        let hinted_ha = wi("は", single(&format!("{}は", KANA_HINT_MOD)));
        assert_eq!(romanize_word_info(&hinted_ha, trad), "wa");
        assert_eq!(romanize_word_info(&hinted_ha, kana), "は");

        let hata = wi("はた", Some(WordInfoKana::Multi(vec![single("はた")])));
        assert_eq!(romanize_word_info(&hata, trad), "hata");
        assert_eq!(romanize_word_info(&hata, kana), "はた");

        let tsu = wi("っ", single("っ"));
        assert_eq!(romanize_word_info(&tsu, trad), "!");
        assert_eq!(romanize_word_info(&tsu, kana), "っ");
        let bar = wi("ー", single("ー"));
        assert_eq!(romanize_word_info(&bar, trad), "~");
        assert_eq!(romanize_word_info(&bar, kana), "ー");
    }

    #[test]
    fn romanize_word_info_method_arm_nil_element() {
        let trad =
            KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
        let wi_nil_elem = wi("x", Some(WordInfoKana::Multi(vec![single("あ"), None])));
        assert_eq!(romanize_word_info(&wi_nil_elem, trad), "a/");
    }

    #[test]
    #[should_panic]
    fn romanize_word_info_kana_arm_nil_element_errors() {
        let wi_nil_elem = wi("x", Some(WordInfoKana::Multi(vec![single("あ"), None])));
        romanize_word_info(&wi_nil_elem, KaniRomanizeMethod::Kana);
    }

    #[test]
    #[should_panic]
    fn romanize_word_info_nested_element_errors() {
        let trad =
            KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
        let wi_nested = wi(
            "x",
            Some(WordInfoKana::Multi(vec![Some(WordInfoKana::Multi(vec![single("あ")]))])),
        );
        romanize_word_info(&wi_nested, trad);
    }

    #[test]
    fn romanize_word_info_nil_kana() {
        let trad =
            KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
        let empty = wi("x", None);
        assert_eq!(romanize_word_info(&empty, trad), "");
        assert_eq!(romanize_word_info(&empty, KaniRomanizeMethod::Kana), "");
    }

    /// REPL fixtures (.103, `ichiran:romanize`), 2026-05-24.
    #[tokio::test]
    async fn romanize_joined_string() {
        let ctx = ctx().await;
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
            let (default_str, _) = romanize(&ctx, input, traditional(), false).await.unwrap();
            assert_eq!(&default_str, expected_default, "default method, input={input:?}");
            let (kana_str, _) = romanize(&ctx, input, KaniRomanizeMethod::Kana, false)
                .await
                .unwrap();
            assert_eq!(&kana_str, expected_kana, ":kana method, input={input:?}");
        }
    }

    #[tokio::test]
    async fn romanize_with_info_collects_definitions_in_order() {
        let ctx = ctx().await;
        let (joined, definitions) = romanize(
            &ctx,
            "富士山は日本で最も高い山である。",
            traditional(),
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
        assert!(definitions[0].1.starts_with("富士山"), "got {:?}", definitions[0].1);
        assert!(definitions[2].1.starts_with("日本"), "got {:?}", definitions[2].1);
        assert!(definitions[6].1.starts_with("山"), "got {:?}", definitions[6].1);
    }

    #[tokio::test]
    async fn romanize_with_info_false_yields_empty_definitions() {
        let ctx = ctx().await;
        let (joined, definitions) =
            romanize(&ctx, "彼女は新しい仮説を提唱した。", traditional(), false)
                .await
                .unwrap();
        assert_eq!(joined, "kanojo wa atarashii kasetsu wo teishō shita. ");
        assert!(definitions.is_empty());
    }

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

    /// REPL fixtures (.103, `ichiran:romanize*`, 2026-05-24).
    #[tokio::test]
    async fn romanize_star_full_structure() {
        let ctx = ctx().await;
        let result = romanize_star_(&ctx, "Hello 世界！", traditional(), None, |_, _| ())
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
        let ctx = ctx().await;
        let result = romanize_star_(&ctx, "ABCは試験的な略語です。", traditional(), None, |_, _| ())
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
        let ctx = ctx().await;
        let result = romanize_star_(&ctx, "世界", traditional(), Some(1), |rom, word| {
            (rom.len(), word.text.clone())
        })
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
}
