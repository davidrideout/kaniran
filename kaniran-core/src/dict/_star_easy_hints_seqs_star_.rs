//! Port of `ichiran/dict:*easy-hints-seqs*` (`dict-split.lisp:904`).
//!
//! Cumulative list of JMdict sequence ids registered by every
//! `def-easy-hint` form in `dict-split.lisp` (lines 1389-1859). The
//! upstream `defparameter` starts at `nil`; each `def-easy-hint`
//! call expands into a `(push ,seq *easy-hints-seqs*)` plus a
//! `(defhint (,seq) ...)` registration into
//! [`super::_star_hint_map_star_::HINT_MAP`]. Since the same
//! callsite populates both globals, this port derives the seq list
//! from [`super::_star_hint_map_star_::EASY_HINTS`] via [`OnceLock`]
//! per CONVENTIONS §5.2 — "build it from them, don't hand-copy."
//!
//! Only consumer is [`super::check_easy_hints::check_easy_hints`] —
//! a test-only sanity-scan that selects `kana_text` rows by these
//! seqs and verifies their `match-readings` shape with hints
//! disabled. Upstream marks the symbol "Only used for testing"
//! (`dict-split.lisp:904` docstring), so this module is gated under
//! `#[cfg(test)]` and absent from release binaries.
//!
//! Order matches the upstream `push` semantics (reverse of
//! source-file order). The check_easy_hints SQL `:in` filter
//! consumes the list as a set, so order doesn't affect behavior;
//! the test below pins the entry count.

use std::sync::OnceLock;

use super::_star_hint_map_star_::EASY_HINTS;

/// Derived from [`EASY_HINTS`] on first access. Mirrors the upstream
/// `(push ,seq *easy-hints-seqs*)` ordering — iteration is in the
/// reverse of source-file order, since `push` prepends.
pub fn easy_hints_seqs() -> &'static [i32] {
    static CACHE: OnceLock<Vec<i32>> = OnceLock::new();
    CACHE.get_or_init(|| {
        // EASY_HINTS is in source-file order (see _star_hint_map_star_).
        // Upstream `push` produces reverse-source order, so reverse here
        // to match the live SBCL image's *easy-hints-seqs* contents.
        EASY_HINTS.iter().rev().map(|e| e.seq).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the entry count against the upstream observation. If a
    /// future upstream `dict-split.lisp` adds or removes a
    /// `def-easy-hint` form, this test fails until [`EASY_HINTS`]
    /// is regenerated.
    #[test]
    fn entry_count_matches_upstream() {
        assert_eq!(easy_hints_seqs().len(), 431);
    }
}
