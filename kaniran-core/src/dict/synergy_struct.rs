//! Port of `ichiran/dict:synergy` (`dict-grammar.lisp:713`).
//!
//! In-memory record describing one inter-word bonus produced by a
//! `defsynergy` rule — a scoring boost applied between two
//! consecutive segments in a parsed path (e.g. noun + particle, na-
//! adjective + な). `description` is the human-readable label,
//! `connector` the kana glue inserted between the two surface forms
//! (often empty), `score` the bonus added to the path total, and
//! `start` / `end` the character span the bonus covers (drawn from
//! the left segment's end and the right segment's start).

// `(defstruct synergy description connector score start end)` has no
// `:initform`s, so every slot defaults to nil. The `description` and
// `connector` slots get bound to strings by most upstream
// `def-generic-synergy` callsites, but a few register synergies that
// leave them nil (encountered in the wi-path bulk corpus). `score`,
// `start`, `end` are always set by the macro expansion to integers.
#[derive(Debug, Clone)]
pub struct Synergy {
    pub description: Option<String>,
    pub connector: Option<String>,
    pub score: i32,
    pub start: usize,
    pub end: usize,
}
