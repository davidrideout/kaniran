//! Port of `ichiran/characters:long-vowel-modifier-p` (`characters.lisp:47-53`).
//!
//! True when a small modifier glyph (`ぁ ィ ぅ ェ ぉ`, classified as
//! `+A/+I/+U/+E/+O`) extends the preceding character's vowel — e.g.
//! `か` followed by `ぁ` produces a long `aa` rather than a `kya`-style
//! fused mora. The check compares the modifier's vowel target against
//! the last character of the previous glyph's `KanaClass` keyword name
//! (e.g. `Ka` → `"KA"`, last char `'A'`).
//!
//! Returns `false` when `modifier` isn't one of the five `+vowel`
//! variants, or when `prev_char` has no known [`KanaClass`]. The Lisp's
//! `(keywordp char-class)` guard is subsumed by [`get_char_class`]
//! returning `Option<KanaClass>` (CONVENTIONS §4.2).

use super::kana_class_tables::get_char_class;
use super::kani_kana_class::KanaClass;

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
