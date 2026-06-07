//! Port of `ichiran:romanize-word` (`romanize.lisp:224-230`).
//!
//! Romanizes a single word: returns the `r-special` mapping when one
//! applies, otherwise processes hints and romanizes its character
//! classes.

use super::generic_romanization_class::RomanizationMethod;
use super::get_character_classes::get_character_classes;
use super::r_special::r_special;
use super::romanize_list::romanize_list;
use crate::characters::kana::NormalizationContext;
use crate::dict::process_hints::process_hints;

pub fn romanize_word(
    word: &str,
    method: RomanizationMethod<'_>,
    original_spelling: Option<&str>,
    normalize: bool,
) -> String {
    // (when normalize (setf word (normalize word))) — normalize is called
    // with no :context, i.e. NormalizationContext::Default.
    let normalized;
    let word: &str = if normalize {
        normalized = crate::characters::kana::normalize(word, NormalizationContext::Default);
        &normalized
    } else {
        word
    };
    // (or (r-special method (or original-spelling word)) …) — empty
    // original-spelling ("") is truthy in CL, so unwrap_or only falls back
    // to word when the keyword is absent.
    if let Some(special) = r_special(method, original_spelling.unwrap_or(word)) {
        return special;
    }
    let word = process_hints(word);
    romanize_list(&get_character_classes(&word), method)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::_star_hepburn_traditional_star_::hepburn_traditional;
    use crate::core::kunrei_siki_class::KunreiSiki;
    use crate::dict::_star_kana_hint_mod_star_::KANA_HINT_MOD;
    use crate::dict::_star_kana_hint_space_star_::KANA_HINT_SPACE;

    #[test]
    fn romanize_word_fixtures() {
        // REPL fixtures (.103, ichiran:romanize-word), 2026-05-24.
        let traditional = RomanizationMethod::TraditionalHepburn(hepburn_traditional());

        // r-special path: lone small tsu / long-vowel bar (method-independent).
        assert_eq!(romanize_word("っ", traditional, None, true), "!");
        assert_eq!(romanize_word("ー", traditional, None, true), "~");

        // normalize=false + original-spelling (the romanize-word-info call shape).
        assert_eq!(
            romanize_word("センター", traditional, Some("センター"), false),
            "senta"
        );
        assert_eq!(romanize_word("よ", traditional, Some("予"), false), "yo");

        // normalize=true: half-width katakana folds to full-width, then romanizes.
        assert_eq!(romanize_word("ｾﾝﾀｰ", traditional, None, true), "senta");
        assert_eq!(romanize_word("こんにちは", traditional, None, true), "konnichiha");

        // original-spelling drives r-special; the word argument is ignored
        // when r-special matches.
        assert_eq!(romanize_word("x", traditional, Some("っ"), false), "!");
        assert_eq!(romanize_word("x", traditional, Some("ー"), false), "~");

        // Empty original-spelling Some("") is truthy: r-special("") is nil,
        // so the word is romanized normally (kanji.lisp:358 call shape).
        assert_eq!(romanize_word("よむ", traditional, Some(""), false), "yomu");
        assert_eq!(romanize_word("しゃしん", traditional, Some(""), false), "shashin");

        // Method variation: kunrei-siki spells し differently from hepburn.
        let kunrei_inst = KunreiSiki::new();
        let kunrei = RomanizationMethod::KunreiSiki(&kunrei_inst);
        assert_eq!(romanize_word("しゃしん", kunrei, None, true), "syasin");
    }

    #[test]
    fn romanize_word_process_hints() {
        // REPL fixtures (.103, ichiran:romanize-word), 2026-05-24 — the
        // process-hints branch operates on the (possibly normalized) word,
        // not on original-spelling.
        let traditional = RomanizationMethod::TraditionalHepburn(hepburn_traditional());

        // *kana-hint-mod* + は → わ → "wa".
        let hint_ha = format!("{}は", KANA_HINT_MOD);
        assert_eq!(romanize_word(&hint_ha, traditional, None, false), "wa");
        // original-spelling "は" is not a special glyph, so it still falls
        // through to process-hints on the hinted word.
        assert_eq!(romanize_word(&hint_ha, traditional, Some("は"), false), "wa");

        // *kana-hint-space* + へ → ASCII space + へ → " he".
        let hint_space = format!("{}へ", KANA_HINT_SPACE);
        assert_eq!(romanize_word(&hint_space, traditional, None, false), " he");
    }
}
