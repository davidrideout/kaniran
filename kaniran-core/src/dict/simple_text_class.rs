//! Port of `ichiran/dict:simple-text` (`dict.lisp:69`).
//!
//! Abstract CLOS base for the [`crate::dict::kanji_text_dao::KanjiText`]
//! and [`crate::dict::kana_text_dao::KanaText`] DAO row classes. The
//! upstream class adds two runtime-mutable state slots that are NOT
//! persisted to the database:
//!
//! - `conjugations` — populated by the find-word pipeline. `None` on
//!   freshly-loaded rows; set to [`WordConjugations::Root`] for original
//!   entries that survived as themselves; set to
//!   [`WordConjugations::Ids`] for derivations carrying a list of
//!   conjugation row ids tying them to their source.
//! - `hintedp` — re-entrance flag for the `:around` `get-kana` method
//!   that interleaves hint generation; `false` by default.
//!
//! Both fields are inlined into the two concrete subclasses
//! (`KanjiText`, `KanaText`) as a [`SimpleText`] composition field
//! initialized to `Default::default()` by their hand-written `FromRow`
//! impls, so DB row decoding leaves the runtime state at its zero
//! values.

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
