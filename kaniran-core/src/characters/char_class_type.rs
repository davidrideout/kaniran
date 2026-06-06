//! Port of `ichiran/characters:char-class` (`characters.lisp:147`).
//!
//! Closed enumeration of the broad character-class tags used to drive
//! regex-based scanning, segmentation, and counting.

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
