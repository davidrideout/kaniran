//! Port of `ichiran/characters:*modifier-characters*`
//! (`characters.lisp:7-9`).
//!
//! Small-form vowels and y-glides used as modifiers (e.g. `ぁ` in
//! `きゃ`), plus the long-vowel mark `ー`. 10 entries. One of the
//! four constituents that `*all-characters*` is built from.

use super::kani_kana_class::KanaClass;

pub static MODIFIER_CHARACTERS: &[(KanaClass, &str)] = &[
    (KanaClass::PlusA, "ぁァ"),
    (KanaClass::PlusI, "ぃィ"),
    (KanaClass::PlusU, "ぅゥ"),
    (KanaClass::PlusE, "ぇェ"),
    (KanaClass::PlusO, "ぉォ"),
    (KanaClass::PlusYa, "ゃャ"),
    (KanaClass::PlusYu, "ゅュ"),
    (KanaClass::PlusYo, "ょョ"),
    (KanaClass::PlusWa, "ゎヮ"),
    (KanaClass::LongVowel, "ー"),
];
