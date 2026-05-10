//! Sidecar enum for the heterogeneous `*split-map*` parts list.
//!
//! No Lisp FQN. The `def-simple-split` macro at `dict-split.lisp:13`
//! pushes three kinds of element into the `parts` accumulator: a word
//! object resolved via `find-word-seq` / `find-word-conj-of`, the bare
//! keyword `:score`, or the bare keyword `:pscore`. The consumer in
//! `calc-score` (`dict.lisp:940-974`) discriminates by `(member :score
//! split)` / `(member :pscore split)` before iterating the remaining
//! word elements. CONVENTIONS §4.3 names that closed tagged shape as
//! a Rust enum.
//!
//! Failed lookups (`find-word-{seq,conj-of}` returning no row) are
//! pushed as Lisp `nil` and modeled as `Option<SplitPart>` at the
//! call site — see [`super::get_split_star::get_split_star`] for the
//! filtering contract.

use super::kani_word::KaniWordDispatchEnum;

#[derive(Debug, Clone)]
pub enum SplitPart {
    Word(KaniWordDispatchEnum),
    Score,
    PScore,
}
