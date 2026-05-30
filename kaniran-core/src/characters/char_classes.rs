//! String/regex constants, the [`CharClass`] enum, the three scanner
//! caches keyed by it, and the predicates that consume them. From
//! `characters.lisp:85-91` (punctuation) and `:106-170` (everything else
//! — constants, mapping, scanners, `count-char-class`, `test-word`).
//!
//! The upstream Lisp recompiles each regex on demand (every callsite
//! passes the raw pattern string into `ppcre:do-matches`). The Rust
//! scanner caches here are an optimization — semantics are identical.

use std::collections::HashMap;
use std::sync::OnceLock;

use fancy_regex::Regex;

// -- string + regex constants (characters.lisp:106-129) -------------------

/// `*abnormal-chars*` — source side of the abnormal→normal map,
/// paired index-by-index with [`NORMAL_CHARS`].
pub static ABNORMAL_CHARS: &str = "\
０１２３４５６７８９\
ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ\
ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ\
＃＄％＆（）＊＋／〈＝〉？＠［］＾＿‘｛｜｝～\
･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// `*normal-chars*` — target side of the abnormal→normal map.
/// Upstream `(concatenate 'string <ASCII prefix> *full-width-kana*)`;
/// captured as a literal here.
pub static NORMAL_CHARS: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// `*full-width-kana*` — paired index-by-index with [`HALF_WIDTH_KANA`].
pub static FULL_WIDTH_KANA: &str =
    "・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// `*half-width-kana*` — paired index-by-index with [`FULL_WIDTH_KANA`].
pub static HALF_WIDTH_KANA: &str =
    "･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// `*decimal-point-regex*`.
pub static DECIMAL_POINT_REGEX: &str = "[.,]";

/// `*digit-regex*` — ASCII, full-width Latin, or ideographic zero `〇`.
pub static DIGIT_REGEX: &str = "[0-9０-９〇]";

/// `*hiragana-regex*` — hiragana + iteration marks `ゝ ゞ` + long-vowel `ー`.
pub static HIRAGANA_REGEX: &str = "[ぁ-ゔゝゞー]";

/// `*kanji-regex*` — CJK ideograph block + `々` + abbreviation marks `ヶ 〆`.
pub static KANJI_REGEX: &str = "[々ヶ〆一-龯]";

/// `*kanji-char-regex*` — CJK ideograph block only (no `々ヶ〆`).
pub static KANJI_CHAR_REGEX: &str = "[一-龯]";

/// `*katakana-regex*` — katakana + iteration marks `ヽ ヾ` + long-vowel `ー`.
pub static KATAKANA_REGEX: &str = "[ァ-ヺヽヾー]";

/// `*katakana-uniq-regex*` — [`KATAKANA_REGEX`] without `ー`.
pub static KATAKANA_UNIQ_REGEX: &str = "[ァ-ヺヽヾ]";

/// `*nonword-regex*` — complement of [`WORD_REGEX`].
pub static NONWORD_REGEX: &str = "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";

/// `*numeric-regex*` — ASCII + full-width digits + ideographic zero +
/// kanji numerals (`一-九` plus traditional / large-unit forms).
pub static NUMERIC_REGEX: &str = "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]";

/// `*num-word-regex*` — [`WORD_REGEX`] plus digits.
pub static NUM_WORD_REGEX: &str = "[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]";

/// `*word-regex*` — kanji + iteration/abbreviation marks + katakana +
/// hiragana + ideographic zero.
pub static WORD_REGEX: &str = "[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";

/// `*punctuation-marks*` (`characters.lisp:85-91`) —
/// `(japanese, ascii-equivalent)` pairs for romanization.
pub static PUNCTUATION_MARKS: &[(&str, &str)] = &[
    ("【", " ["),
    ("】", "] "),
    ("、", ", "),
    ("，", ", "),
    ("。", ". "),
    ("・・・", "... "),
    ("・", " "),
    ("　", " "),
    ("「", " \""),
    ("」", "\" "),
    ("゛", "\""),
    ("『", " «"),
    ("』", "» "),
    ("〜", " - "),
    ("：", ": "),
    ("！", "! "),
    ("？", "? "),
    ("；", "; "),
];

// -- CharClass + mapping + scanners (characters.lisp:131-155) -------------

/// `(deftype char-class ...)` — variant order matches the upstream
/// `member` list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CharClass {
    Katakana,
    KatakanaUniq,
    Hiragana,
    Kanji,
    KanjiChar,
    Kana,
    Traditional,
    Nonword,
    Number,
}

/// `*char-class-regex-mapping*` — one-character pattern per class.
pub static CHAR_CLASS_REGEX_MAPPING: &[(CharClass, &str)] = &[
    (CharClass::Katakana, "[ァ-ヺヽヾー]"),
    (CharClass::KatakanaUniq, "[ァ-ヺヽヾ]"),
    (CharClass::Hiragana, "[ぁ-ゔゝゞー]"),
    (CharClass::Kanji, "[々ヶ〆一-龯]"),
    (CharClass::KanjiChar, "[一-龯]"),
    (CharClass::Kana, "([ァ-ヺヽヾー]|[ぁ-ゔゝゞー])"),
    (CharClass::Traditional, "([ぁ-ゔゝゞー]|[々ヶ〆一-龯])"),
    (CharClass::Nonword, "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]"),
    (
        CharClass::Number,
        "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]",
    ),
];

/// `*char-scanners*` — `^pat+$`.
pub fn char_scanners() -> &'static HashMap<CharClass, Regex> {
    static CACHE: OnceLock<HashMap<CharClass, Regex>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            let wrapped = format!("^{pat}+$");
            let re = Regex::new(&wrapped)
                .unwrap_or_else(|e| panic!("class {class:?} pattern {wrapped:?} failed: {e}"));
            h.insert(*class, re);
        }
        h
    })
}

/// `*char-scanners-inner*` — `(?:pat)+`.
pub fn char_scanners_inner() -> &'static HashMap<CharClass, Regex> {
    static CACHE: OnceLock<HashMap<CharClass, Regex>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            let wrapped = format!("(?:{pat})+");
            let re = Regex::new(&wrapped)
                .unwrap_or_else(|e| panic!("class {class:?} pattern {wrapped:?} failed: {e}"));
            h.insert(*class, re);
        }
        h
    })
}

/// Bare scanners — one [`Regex`] per [`CharClass`] using the pattern
/// from [`CHAR_CLASS_REGEX_MAPPING`] directly (no `+` repetition
/// wrapper). Used by [`count_char_class`] for single-character matches.
/// Rust-only sidecar; upstream Lisp uses the raw pattern string each
/// time and skips the cache.
pub fn char_class_bare_scanners() -> &'static HashMap<CharClass, Regex> {
    static CACHE: OnceLock<HashMap<CharClass, Regex>> = OnceLock::new();
    CACHE.get_or_init(|| {
        CHAR_CLASS_REGEX_MAPPING
            .iter()
            .map(|(c, pat)| {
                let re = Regex::new(pat)
                    .unwrap_or_else(|e| panic!("class {c:?} pattern {pat:?} failed: {e}"));
                (*c, re)
            })
            .collect()
    })
}

/// `*basic-split-regex*` — composite tokenizer for
/// Japanese-mixed-with-digits text, built from the regex constants
/// above via the upstream `format` template.
pub fn basic_split_regex() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| {
        format!(
            "((?:(?<!{}|{}){}+|{}){}*{}|{})",
            DECIMAL_POINT_REGEX,
            DIGIT_REGEX,
            DIGIT_REGEX,
            WORD_REGEX,
            NUM_WORD_REGEX,
            WORD_REGEX,
            WORD_REGEX,
        )
    })
}

// -- predicates (characters.lisp:160-170) ---------------------------------

/// `test-word` (`characters.lisp:160-163`). True iff every character of
/// `word` belongs to `char_class` — the scanner from [`char_scanners`]
/// is anchored as `^pat+$`, so any non-class character makes the match
/// fail. The Lisp returns the match start position (truthy) or nil
/// (falsy); every caller treats it as a predicate, so the Rust signature
/// is `bool`.
pub fn test_word(word: &str, char_class: CharClass) -> bool {
    char_scanners()
        .get(&char_class)
        .expect("char_class is in *char-scanners*")
        .is_match(word)
        .unwrap_or(false)
}

/// `count-char-class` (`characters.lisp:165-170`). Non-overlapping
/// matches of `char_class`'s pattern in `word`.
pub fn count_char_class(word: &str, char_class: CharClass) -> usize {
    char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*")
        .find_iter(word)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Abnormal/normal pairing is index-by-index; lengths must match.
    #[test]
    fn abnormal_and_normal_have_equal_char_count() {
        assert_eq!(ABNORMAL_CHARS.chars().count(), NORMAL_CHARS.chars().count());
    }

    /// Same for the kana-context source/target.
    #[test]
    fn half_and_full_width_kana_have_equal_char_count() {
        assert_eq!(
            HALF_WIDTH_KANA.chars().count(),
            FULL_WIDTH_KANA.chars().count()
        );
    }

    #[test]
    fn every_mapping_pattern_compiles_under_fancy_regex() {
        for (class, pat) in CHAR_CLASS_REGEX_MAPPING {
            Regex::new(pat)
                .unwrap_or_else(|e| panic!("class {class:?} regex {pat:?} failed: {e}"));
        }
    }

    #[test]
    fn char_scanners_cover_every_class_in_the_mapping() {
        let h = char_scanners();
        for (class, _) in CHAR_CLASS_REGEX_MAPPING {
            assert!(h.contains_key(class), "missing scanner for {class:?}");
        }
    }

    #[test]
    fn char_scanners_inner_cover_every_class_in_the_mapping() {
        let h = char_scanners_inner();
        for (class, _) in CHAR_CLASS_REGEX_MAPPING {
            assert!(h.contains_key(class), "missing inner scanner for {class:?}");
        }
    }

    #[test]
    fn char_class_bare_scanners_cover_every_class_in_the_mapping() {
        let h = char_class_bare_scanners();
        for (class, _) in CHAR_CLASS_REGEX_MAPPING {
            assert!(h.contains_key(class), "missing scanner for {class:?}");
        }
    }

    #[test]
    fn basic_split_regex_compiles_under_fancy_regex() {
        Regex::new(basic_split_regex()).expect("regex must compile");
    }

    /// Pinned against the Lisp introspector's captured value.
    #[test]
    fn basic_split_regex_matches_introspected_value() {
        assert_eq!(
            basic_split_regex(),
            "((?:(?<![.,]|[0-9０-９〇])[0-9０-９〇]+|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]*[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])"
        );
    }
}
