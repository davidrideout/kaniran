//! Port of `ichiran/dict-split.lisp` — the JMdict seq → split-fn and
//! seq → hint-fn registries (`*split-map*`, `*segsplit-map*`,
//! `*hint-map*`) plus the runtime engines that look up and apply them.
//!
//! Five sections mirror the upstream's internal structure:
//!
//! - [`split_map`] — the 174-entry `*split-map*` registry, populated
//!   by every `def-simple-split` / `def-de-split` / `def-toori-split`
//!   / `def-do-split` / `def-shi-split` callsite.
//! - [`split`] — `get-split*` / `get-split` dispatchers and the
//!   `optprefix` closure factory.
//! - [`segsplit`] — the 18-entry `*segsplit-map*` (rebind overlay of
//!   `*split-map*`) and the `get-segsplit` compound-text wrapper.
//! - [`hint_map`] — `*hint-map*` (`def-easy-hint` + `def-simple-hint`
//!   bodies) plus the four character/string globals
//!   (`*kana-hint-space*`, `*kana-hint-mod*`, `*hint-char-map*`,
//!   `*hint-simplify-map*`).
//! - [`hint`] — `process-hints` / `strip-hints` / `insert-hints` /
//!   `translate-hint-position` / `translate-hints` / `get-hint`, plus
//!   the test-only `*hints-checked*` / `*easy-hints-seqs*` /
//!   `check-easy-hints` audit triplet.

pub mod hint;
pub mod hint_map;
pub mod segsplit;
pub mod split;
pub mod split_map;
