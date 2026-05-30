//! Port of the `ichiran/characters` Lisp package.
//!
//! Upstream `characters.lisp` is one file; the Rust port splits it into
//! six modules that mirror the upstream's internal section order:
//!
//! - [`kana_class`] — `KanaClass`, kana/sokuon/iteration/modifier
//!   tables, the per-glyph reverse lookup, `long-vowel-modifier-p`.
//! - [`voicing`] — dakuten/handakuten/undakuten tables, dakuten-join,
//!   `voice-char`, `rendaku`, `unrendaku`, `geminate`.
//! - [`char_classes`] — string/regex constants, punctuation pairs,
//!   `CharClass`, the three scanner caches, `basic-split-regex`,
//!   `count-char-class`, `test-word`.
//! - [`kanji`] — `kanji-mask`, `kanji-regex`, `kanji-match`,
//!   `kanji-cross-match`, `kanji-prefix`, `sequential-kanji-positions`.
//! - [`normalize`] — `to-normal-char`, `normalize`, `simplify-ngrams`,
//!   `as-hiragana`, `as-katakana`.
//! - [`text_utils`] — `split-by-regex`, `basic-split`,
//!   `consecutive-char-groups`, `mora-length`, `destem`, `match-diff`,
//!   `safe-subseq`, `join`.

pub mod char_classes;
pub mod kana_class;
pub mod kanji;
pub mod normalize;
pub mod text_utils;
pub mod voicing;
