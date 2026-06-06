//! Port of `ichiran/dict:compound-text` (`dict.lisp:608`).
//!
//! In-memory record for a compound token — the runtime aggregate the
//! segmenter builds via `adjoin-word` when it chains adjacent readings
//! into one word.

use crate::dict::kani_word::KaniWordDispatchEnum;

#[derive(Debug, Clone)]
pub struct CompoundText {
    pub text: String,
    pub kana: String,
    pub primary: Box<KaniWordDispatchEnum>,
    pub words: Vec<KaniWordDispatchEnum>,
    pub score_base: Option<Box<KaniWordDispatchEnum>>,
    pub score_mod: ScoreMod,
}

/// Three variants matching the three reachable methods of
/// `(defgeneric apply-score-mod …)` at `dict.lisp:735-742`:
///
/// - [`ScoreMod::Single`] — the upstream `((score-mod integer) …)`
///   method's input shape. `apply-score-mod` computes
///   `score * sm * len`.
/// - [`ScoreMod::Constant`] — the upstream `((score-mod function) …)`
///   method's only reachable input shape: `(constantly N)` from
///   `dict-grammar.lisp:404, 448, 516, 532`. `apply-score-mod`
///   computes `(funcall (constantly N) score)` → `N`. The Rust
///   variant carries `N` directly because no upstream callsite ever
///   constructs a non-`constantly` closure as `:score-mod`.
/// - [`ScoreMod::Stack`] — the upstream `((score-mod list) …)`
///   method's input shape, holding a flat list whose elements are
///   either `Single` or `Constant` (built by `adjoin-word`'s
///   cons/list growth at `dict.lisp:651`).
///
/// Payloads are `i64` to match SBCL's 63-bit fixnum width — `score`,
/// `len`, every stored `score-mod` literal, and the multiplication
/// chain in `apply-score-mod` all participate in the same numeric
/// type, with no narrowing at the function boundary.
#[derive(Debug, Clone)]
pub enum ScoreMod {
    Single(i64),
    Constant(i64),
    Stack(Vec<ScoreMod>),
}
