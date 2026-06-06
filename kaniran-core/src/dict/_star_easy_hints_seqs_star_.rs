//! Port of `ichiran/dict:*easy-hints-seqs*` (`dict-split.lisp:904`).
//!
//! List of JMdict sequence ids registered by every `def-easy-hint`
//! form. Upstream marks it "Only used for testing", so this module is
//! gated under `#[cfg(test)]` and absent from release binaries.

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
