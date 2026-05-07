//! Port of `ichiran:kana-representation` (`deromanize.lisp:23`).
//!
//! In-memory record carrying one branch of the deromanizer's
//! candidate tree — the partial kana built so far (`canonical`), the
//! original romaji pattern that produced it (`pattern`), the romaji
//! still to consume (`rest`), and a per-branch tie-breaker tag
//! (`branch`) used to disambiguate sibling candidates in the search.
//! All four slots default to the empty value (`""` / `0`) per the
//! upstream `(:default "")` / `(:default 0)` initforms.

#[derive(Debug, Clone, Default)]
pub struct KanaRepresentation {
    pub canonical: String,
    pub pattern: String,
    pub rest: String,
    pub branch: i32,
}
