//! Rust-only sidecar (CONVENTIONS §1, §2): the interpreter that drives
//! every `def-simple-split` callsite.
//!
//! No Lisp FQN. The macro `def-simple-split` (`dict-split.lisp:13`)
//! and its derived helpers (`def-de-split`, `def-toori-split`,
//! `def-do-split`, `def-shi-split`) expand each call into a hand-rolled
//! `prog*` loop with the same skeleton: materialize `(true-text reading)`
//! and its length, walk a sequence of part / `:test` / `:score` / `:pscore`
//! forms, and return `(values parts score)`. The expansion is identical
//! up to the per-callsite arguments (seq, score, parts list) — which
//! makes a per-callsite Rust function just a hand-expansion of the same
//! template.
//!
//! This module factors that template out. Every split-* callsite ports
//! to a [`SplitDef`] data row (in the callsite's own
//! `dict/split_*.rs` so CONVENTIONS §1's "one Lisp symbol per Rust file"
//! still holds, and so [`crate::dict::_star_split_map_star_`] / the
//! audit-signatures sweep see the expected `pub fn`); the function body
//! is one line that delegates to [`run_split`]. Any future
//! `def-simple-split` Rust port adds a row, not a 50-line block of
//! scaffolding.
//!
//! See [`crate::dict::_star_split_map_star_`] for the dispatch table
//! that enumerates every registered seq, and the `def-simple-split`
//! macro doc-comment in `dict-split.lisp:13` for the reference
//! expansion shape.

use crate::characters::text_utils::safe_subseq;
use crate::characters::voicing::unrendaku as unrendaku_fn;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_conj_of::find_word_conj_of;
use crate::dict::find_word_seq::find_word_seq;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;
use crate::dict::optprefix::optprefix;
use crate::dict::word_type::WordType;

/// One registered split callsite — the macro arguments collapsed into
/// data. Every `dict/split_*.rs` exposes one of these as `pub static`
/// and a 3-line `pub async fn` shim that calls [`run_split`] on it.
pub struct SplitDef {
    /// JMdict seq the upstream `def-simple-split` registers under.
    pub seq: i32,
    /// Initial value of the `score-var` `prog*` binding — returned as
    /// the second `(values ...)` slot when the loop completes without
    /// hitting a `:test` rejection.
    pub score: i32,
    /// Body forms in macro-callsite order. Mirrors the iteration of
    /// the `parts-def` `&body` argument inside the macro
    /// `(loop for (part-seq part-length-form conj-p modify) in parts-def ...)`.
    pub steps: &'static [Step],
}

/// One element of the `parts-def` `&body` — the macro discriminates on
/// the first element to decide which branch of the `cond` to emit.
pub enum Step {
    /// `(:test <expr> [<score-modify>] [<modify>])` — the macro emits
    /// `(unless ,part-length-form ,@(setf score) ,@(push modify) (go :end))`.
    Test {
        pred: Pred,
        /// Optional `score-modify` (3rd parts-def slot when the first
        /// is `:test`) — replaces `score-var` if the test fails.
        score_mod: Option<i32>,
        /// Optional `modify` (4th parts-def slot when the first is
        /// `:test`) — pushed onto `parts` if the test fails.
        push: Option<ScorePush>,
    },
    /// `:score` / `:pscore` standalone — the `(find part-seq '(:score :pscore))`
    /// branch. Pushes the keyword onto `parts` unconditionally.
    Push(ScorePush),
    /// Normal `(part-seq part-length-form [conj-p] [modify])` form.
    Word(WordPart),
}

/// Shape of `:test` predicates observed in the existing `def-simple-split`
/// callsites. The macro is fully general (any expression), but every
/// callsite uses one of these seven shapes; [`Pred::Compute`] is the
/// escape hatch for one-offs.
pub enum Pred {
    /// `(eql (word-type r) :kana)` / `:kanji`
    WordType(WordType),
    /// `(equal txt "<lit>")`
    TextEquals(&'static str),
    /// `(alexandria:starts-with-subseq "<lit>" txt)`
    TextStartsWith(&'static str),
    /// `(> len N)` — strictly greater
    LenGt(i32),
    /// `(= len N)`
    LenEq(i32),
    /// One-off predicate; `fn(txt, reading, len_) -> bool`.
    Compute(fn(&str, &KaniSimpleTextDispatchEnum, usize) -> bool),
}

#[derive(Clone, Copy)]
pub enum ScorePush {
    Score,
    PScore,
}

/// Normal word-part form. The macro generates pseq via
/// `(if (listp part-seq) (if (and ... (stringp (car ...))) ...))` —
/// the three cases collapse here as [`PartSeq`].
pub struct WordPart {
    pub seq: PartSeq,
    pub length: Len,
    pub finder: Finder,
    pub modify: Modify,
}

/// `part-seq` shape. Matches the macro's three cases:
/// - bare integer → wrapped as 1-element list
/// - quoted list of integers → multi-element static
/// - `("<text>" <seq>)` → dynamic — resolved at runtime via
///   `find-word-conj-of` / `seq` of `car`
pub enum PartSeq {
    Static(&'static [i32]),
    /// `(seq (car (find-word-conj-of <text> <seq>)))`
    Dynamic { text: &'static str, seq: i32 },
}

/// `part-length-form` shape. The macro accepts any expression; this
/// enum names the patterns observed across the registered callsites
/// and falls back to [`Len::Compute`] for one-offs.
pub enum Len {
    /// `nil` / open part — `safe-subseq` runs to end of `txt`.
    Open,
    Fixed(usize),
    /// `(- len N)` clamped to 0 (matches existing port shape
    /// `((len_ as i32 - N).max(0) as usize)`).
    LenMinus(usize),
    /// `(position #\<c> txt)` — `None` if char not found.
    CharPos(char),
    /// `(1+ (position #\<c> txt))` — `None` if char not found,
    /// otherwise position+1.
    CharPosPlus1(char),
    /// One-off length form; `fn(txt, len_) -> Option<usize>`.
    Compute(fn(&str, usize) -> Option<usize>),
}

#[derive(Clone, Copy)]
pub enum Finder {
    /// `(car (apply 'find-word-seq ...))` — `conj-p` slot is nil.
    Seq,
    /// `(car (apply 'find-word-conj-of ...))` — `conj-p` slot is `t`.
    ConjOf,
}

/// `modify` slot (4th of `parts-def` for word forms). The macro:
///   `(case modify ((t) `(unrendaku ,part-txt))
///                  ((nil) part-txt)
///                  (t `(funcall ,modify ,part-txt)))`.
pub enum Modify {
    None,
    /// modify = `t` → `(unrendaku part-txt)`
    Unrendaku,
    /// modify = `(optprefix "<lit>")` — applies the [`optprefix`]
    /// closure factory to `part-txt`. Only existing observed `funcall`
    /// case.
    OptPrefix(&'static str),
}

impl Pred {
    fn eval(&self, txt: &str, reading: &KaniSimpleTextDispatchEnum, len_: usize) -> bool {
        match self {
            Pred::WordType(wt) => reading.word_type() == *wt,
            Pred::TextEquals(s) => txt == *s,
            Pred::TextStartsWith(s) => txt.starts_with(*s),
            Pred::LenGt(n) => len_ as i32 > *n,
            Pred::LenEq(n) => len_ as i32 == *n,
            Pred::Compute(f) => f(txt, reading, len_),
        }
    }
}

impl ScorePush {
    fn to_part(&self) -> SplitPart {
        match self {
            ScorePush::Score => SplitPart::Score,
            ScorePush::PScore => SplitPart::PScore,
        }
    }
}

impl Len {
    fn eval(&self, txt: &str, len_: usize) -> Option<usize> {
        match self {
            Len::Open => None,
            Len::Fixed(n) => Some(*n),
            Len::LenMinus(n) => Some(((len_ as i32 - *n as i32).max(0)) as usize),
            Len::CharPos(c) => txt.chars().position(|x| x == *c),
            Len::CharPosPlus1(c) => txt.chars().position(|x| x == *c).map(|p| p + 1),
            Len::Compute(f) => f(txt, len_),
        }
    }
}

impl Modify {
    fn apply(&self, pt: &str) -> String {
        match self {
            Modify::None => pt.to_string(),
            Modify::Unrendaku => {
                let mut s = pt.to_string();
                unrendaku_fn(&mut s);
                s
            }
            Modify::OptPrefix(prefix) => optprefix(prefix)(pt),
        }
    }
}

async fn resolve_pseq(
    ctx: &KaniranContext,
    pseq: &PartSeq,
) -> Result<Vec<i32>, sqlx::Error> {
    match pseq {
        PartSeq::Static(s) => Ok(s.to_vec()),
        PartSeq::Dynamic { text, seq } => {
            let lookup = find_word_conj_of(ctx, text, &[*seq]).await?;
            Ok(lookup.first_seq().into_iter().collect())
        }
    }
}

/// Interprets a [`SplitDef`] against a reading. Mirrors the `prog*`
/// expansion of `def-simple-split` (`dict-split.lisp:13`) statement-by-
/// statement: the `:end` `go` target is encoded as an early `Ok`
/// return.
pub async fn run_split(
    def: &SplitDef,
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
) -> Result<(Vec<Option<SplitPart>>, i32), sqlx::Error> {
    let txt: String = reading.true_text().to_string();
    let len_: usize = txt.chars().count();
    let mut offset: usize = 0;
    let mut parts: Vec<Option<SplitPart>> = Vec::new();
    let mut score: i32 = def.score;

    for step in def.steps {
        match step {
            Step::Test { pred, score_mod, push } => {
                if !pred.eval(&txt, reading, len_) {
                    if let Some(s) = score_mod {
                        score = *s;
                    }
                    if let Some(p) = push {
                        parts.push(Some(p.to_part()));
                    }
                    return Ok((parts, score));
                }
            }
            Step::Push(p) => {
                parts.push(Some(p.to_part()));
            }
            Step::Word(w) => {
                let pseq_vec = resolve_pseq(ctx, &w.seq).await?;
                let part_length = w.length.eval(&txt, len_);
                let part_txt = safe_subseq(&txt, offset, part_length.map(|pl| offset + pl));
                let pushed: Option<SplitPart> = if pseq_vec.contains(&def.seq) {
                    None
                } else if let Some(pt) = part_txt {
                    let pt_modified = w.modify.apply(&pt);
                    match w.finder {
                        Finder::Seq => find_word_seq(ctx, &pt_modified, &pseq_vec)
                            .await?
                            .first_word()
                            .map(SplitPart::Word),
                        Finder::ConjOf => find_word_conj_of(ctx, &pt_modified, &pseq_vec)
                            .await?
                            .first_word()
                            .map(SplitPart::Word),
                    }
                } else {
                    None
                };
                parts.push(pushed);
                if let Some(pl) = part_length {
                    offset += pl;
                }
            }
        }
    }

    Ok((parts, score))
}
