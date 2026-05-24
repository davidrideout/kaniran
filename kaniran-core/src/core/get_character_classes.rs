//! Port of `ichiran:get-character-classes` (`romanize.lisp:5-7`).
//!
//! Maps each character of `word` to its character class, looking the
//! glyph up in `*char-class-hash*` and falling back to the glyph itself
//! when it is not kana. The result feeds `process-iteration-characters`
//! and `process-modifiers`.

use super::kani_cc_item::CcItem;
use crate::characters::_star_char_class_hash_star_::char_class_hash;

pub fn get_character_classes(word: &str) -> Vec<CcItem> {
    word.chars()
        .map(|char| match char_class_hash().get(&char) {
            Some(&class) => CcItem::Class(class),
            None => CcItem::Char(char),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kani_kana_class::KanaClass;

    fn class(kana: KanaClass) -> CcItem {
        CcItem::Class(kana)
    }

    #[test]
    fn get_character_classes_fixtures() {
        use KanaClass::*;
        // REPL fixtures (.103, ichiran::get-character-classes), 2026-05-23.
        let cases: Vec<(&str, Vec<CcItem>)> = vec![
            ("し", vec![class(Shi)]),
            ("による", vec![class(Ni), class(Yo), class(Ru)]),
            // long-vowel modifier
            ("コーヒー", vec![class(Ko), class(LongVowel), class(Hi), class(LongVowel)]),
            // sokuon
            ("きっぷ", vec![class(Ki), class(Sokuon), class(Pu)]),
            // iteration marks
            ("ゝゞ", vec![class(Iter), class(IterV)]),
            // non-kana glyphs return the char itself
            ("Aと5", vec![CcItem::Char('A'), class(To), CcItem::Char('5')]),
            // kanji all fall back to chars
            ("東京", vec![CcItem::Char('東'), CcItem::Char('京')]),
        ];
        for (word, expected) in &cases {
            assert_eq!(&get_character_classes(word), expected, "word={word:?}");
        }
    }
}
