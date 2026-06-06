//! Port of `ichiran/dict:synergy` (`dict-grammar.lisp:713`).
//!
//! In-memory record describing one inter-word scoring bonus applied
//! between two consecutive segments in a parsed path (e.g. noun +
//! particle, na-adjective + な).

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
