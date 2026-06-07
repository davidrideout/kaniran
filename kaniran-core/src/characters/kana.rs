use super::char_class::{get_char_class, simplify_ngrams, CharClass};
use super::constants::{ABNORMAL_CHARS, FULL_WIDTH_KANA, HALF_WIDTH_KANA, PUNCTUATION_MARKS};
use super::helpers::{all_characters, char_class_hash, dakuten_join, normal_chars};
use super::kani_char_class_bare_scanners::char_class_bare_scanners;
use super::kani_kana_class::KanaClass;

/// Port of `ichiran/characters:long-vowel-modifier-p` (`characters.lisp:47-53`).
///
/// True when a small modifier glyph (`ぁ ィ ぅ ェ ぉ`, classified as
/// `+A/+I/+U/+E/+O`) extends the preceding character's vowel — e.g.
/// `か` followed by `ぁ` produces a long `aa` rather than a `kya`-style
/// fused mora.
///
/// Returns `false` when `modifier` isn't one of the five `+vowel`
/// variants, or when `prev_char` has no known [`KanaClass`].
pub fn long_vowel_modifier_p(modifier: KanaClass, prev_char: char) -> bool {
    let vowel = match modifier {
        KanaClass::PlusA => 'A',
        KanaClass::PlusI => 'I',
        KanaClass::PlusU => 'U',
        KanaClass::PlusE => 'E',
        KanaClass::PlusO => 'O',
        _ => return false,
    };
    let Some(class) = get_char_class(prev_char) else {
        return false;
    };
    class.lisp_name().chars().last() == Some(vowel)
}

/// Port of `ichiran/characters:to-normal-char` (`characters.lisp:219-222`).
///
/// Map a single character through the abnormal→normal substitution
/// tables. With [`NormalizationContext::Default`], the source/target
/// pair is `*abnormal-chars*` → `*normal-chars*` (full-width ASCII /
/// half-width katakana → standard ASCII / full-width katakana). With
/// [`NormalizationContext::Kana`], it's `*half-width-kana*` →
/// `*full-width-kana*` only — used by callers that want to normalize
/// half-width katakana but leave ASCII decorations alone.
///
/// Returns `None` when the input is not in the relevant source table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationContext {
    Default,
    Kana,
}

pub fn to_normal_char(c: char, context: NormalizationContext) -> Option<char> {
    let (src, dst): (&str, &str) = match context {
        NormalizationContext::Kana => (HALF_WIDTH_KANA, FULL_WIDTH_KANA),
        NormalizationContext::Default => (ABNORMAL_CHARS, normal_chars()),
    };
    let pos = src.chars().position(|x| x == c)?;
    dst.chars().nth(pos)
}

/// Port of `ichiran/characters:normalize` (`characters.lisp:224-232`).
///
/// Convert abnormal-but-Japanese-rendered ASCII (full-width digits and
/// punctuation, half-width katakana) back to plain ASCII / full-width
/// katakana, then collapse combining-mark sequences (`か゛ → が`) and —
/// outside `:kana` mode — Japanese punctuation runs (`、 → ", "`).
pub fn normalize(s: &str, context: NormalizationContext) -> String {
    let phase1: String = s
        .chars()
        .map(|c| to_normal_char(c, context).unwrap_or(c))
        .collect();
    match context {
        NormalizationContext::Kana => simplify_ngrams(&phase1, dakuten_join()),
        NormalizationContext::Default => {
            let combined: Vec<(&str, &str)> = PUNCTUATION_MARKS
                .iter()
                .copied()
                .chain(dakuten_join().iter().map(|(a, b)| (a.as_str(), b.as_str())))
                .collect();
            simplify_ngrams(&phase1, &combined)
        }
    }
}

/// Port of `ichiran/characters:mora-length` (`characters.lisp:245-249`).
///
/// Counts the number of "real" morae in a kana string, ignoring the
/// sokuon, all small kana modifiers (`ぁィゥェォ`, `ャュョ`), and the
/// long-vowel mark `ー`. Each excluded glyph either fuses with or
/// lengthens its neighbour rather than contributing a mora of its own.
const MODIFIERS: &str = "っッぁァぃィぅゥぇェぉォゃャゅュょョー";

pub fn mora_length(s: &str) -> usize {
    s.chars().filter(|c| !MODIFIERS.contains(*c)).count()
}

/// Port of `ichiran/characters:as-hiragana` (`characters.lisp:251-260`).
///
/// Convert any katakana in `s` to its hiragana counterpart, leaving
/// non-kana characters as-is.
pub fn as_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| {
            let c = to_normal_char(c, NormalizationContext::Default).unwrap_or(c);
            match char_class_hash().get(&c) {
                Some(&class) => first_char_for(class),
                None => c,
            }
        })
        .collect()
}

fn first_char_for(class: KanaClass) -> char {
    all_characters()
        .iter()
        .find(|(k, _)| *k == class)
        .expect("class from char-class-hash must be in all-characters")
        .1
        .chars()
        .next()
        .expect("class string is non-empty")
}

/// Port of `ichiran/characters:as-katakana` (`characters.lisp:262-271`).
///
/// Convert any hiragana in `s` to its katakana counterpart, leaving
/// non-kana characters as-is.
pub fn as_katakana(s: &str) -> String {
    s.chars()
        .map(|c| {
            let c = to_normal_char(c, NormalizationContext::Default).unwrap_or(c);
            match char_class_hash().get(&c) {
                Some(&class) => last_char_for(class),
                None => c,
            }
        })
        .collect()
}

fn last_char_for(class: KanaClass) -> char {
    all_characters()
        .iter()
        .find(|(k, _)| *k == class)
        .expect("class from char-class-hash must be in all-characters")
        .1
        .chars()
        .last()
        .expect("class string is non-empty")
}

/// Port of `ichiran/characters:destem` (`characters.lisp:316-324`).
///
/// Trim from the end of `word` the suffix that begins at the
/// `stem`-th match of `char_class`'s pattern (counted from the end).
/// Fewer than `stem` matches yields an empty result; the cut falls on
/// a character boundary.
pub fn destem(word: &str, stem: usize, char_class: CharClass) -> String {
    if stem == 0 {
        return word.to_string();
    }
    let re = char_class_bare_scanners()
        .get(&char_class)
        .expect("char_class is in *char-class-regex-mapping*");
    let positions: Vec<usize> = re
        .find_iter(word)
        .map(|m| m.expect("regex iteration"))
        .map(|m| word[..m.start()].chars().count())
        .collect();
    if stem > positions.len() {
        return String::new();
    }
    let cut = positions[positions.len() - stem];
    word.chars().take(cut).collect()
}

#[cfg(test)]
mod tests;
