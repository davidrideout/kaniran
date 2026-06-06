//! Port of `ichiran/dict:simple-text` (`dict.lisp:69`).
//!
//! Abstract CLOS base for the [`crate::dict::kanji_text_dao::KanjiText`]
//! and [`crate::dict::kana_text_dao::KanaText`] DAO row classes,
//! holding the two runtime-mutable state slots that are not persisted.

#[derive(Debug, Clone, Default)]
pub struct SimpleText {
    pub conjugations: Option<WordConjugations>,
    pub hintedp: bool,
}

/// Value space of the upstream `conjugations` slot. The Lisp slot can
/// hold `nil` (no annotation), the symbol `:root` (entry kept as
/// itself), or a list of integer conjugation ids tying a derived
/// reading back to the conjugation rows that produced it. The `nil`
/// case is the `None` of `Option<WordConjugations>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordConjugations {
    Root,
    Ids(Vec<i32>),
}
