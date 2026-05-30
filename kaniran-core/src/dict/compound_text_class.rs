//! Port of `ichiran/dict:compound-text` (`dict.lisp:608`).
//!
//! In-memory record for two-or-more-words-squished-together
//! tokens — the runtime aggregate produced by `adjoin-word`
//! (`dict.lisp:632-651`) when the segmenter chains adjacent readings
//! into a compound. Distinct from the `simple-text` family: it is
//! NOT a `simple-text` subclass, has no `conjugations` / `hintedp`
//! state, and most identity-bearing generics (`common`, `ord`,
//! `word-type`, `seq`) delegate to the wrapped child via `primary` or
//! the last word in `words`.
//!
//! Slot shape:
//! - `text`, `kana` — surface forms of the compound (concatenated by
//!   `adjoin-word`'s `:around` method, `dict.lisp:635-640`).
//! - `primary` — the head reading whose identity bleeds through
//!   (`(common (primary obj))`, `(ord (primary obj))`, etc.).
//! - `words` — the full sequence of children in order. `primary`
//!   is set to the first child at construction; `words` grows by
//!   appending on subsequent `adjoin-word` calls
//!   (`dict.lisp:647-651`).
//! - `score_base` — optional word whose identity drives the
//!   recursive `calc-score` call when the compound is scored
//!   (`dict.lisp:782` consumes it via the `score-base` reader at
//!   `dict.lisp:669-672`, which falls back to `primary` when the
//!   slot is `nil`). `def-simple-suffix` sets it from
//!   `(second pw)` where `pw` is a `(word_g0 word_g1)` pair from
//!   `pair-words-by-conj` (`dict-grammar.lisp:351-367, 58-73`),
//!   so the slot holds a word, not an integer.
//! - `score_mod` — score modifier accumulated across adjoins. The
//!   first `adjoin-word` writes a single value; the second wraps it
//!   `(list new old)`; further adjoins `(cons new old-list)`, leaving
//!   a flat list. Each element is either an integer (`:score-mod 5`
//!   literal at most `def-simple-suffix` callsites) or a
//!   `(constantly N)` closure (four upstream callsites:
//!   `suffix-kudasai` / `suffix-sou` / `suffix-desu` / `suffix-desho`
//!   at `dict-grammar.lisp:404, 448, 516, 532`). `apply-score-mod`
//!   dispatches on the element type: integers compute
//!   `score * sm * len`, closures evaluate `(funcall sm score)` which
//!   for `(constantly N)` just returns `N` flat. Modeled as
//!   [`ScoreMod`] per CONVENTIONS §4.3.
//!
//! Lisp `primary` and `words` are `t`-typed; in practice all
//! callsites set them to `simple-text` instances (the
//! `(simple-text simple-text)` and `(compound-text simple-text)`
//! `adjoin-word` overloads are the only producers). The port keeps
//! the slot type open as [`Box<KaniWordDispatchEnum>`] /
//! [`Vec<KaniWordDispatchEnum>`] to mirror the Lisp permissiveness
//! and avoid a future widening if a new `adjoin-word` overload lands.
//!
//! No methods are ported here — `seq`, `common`, `ord`, `word-type`,
//! `text`, `get-kana`, `score-base`, `word-conj-data`, and
//! `word-conjugations` for compound-text all land with the wave-158
//! SCC.

use crate::dict::kani::KaniWordDispatchEnum;

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
