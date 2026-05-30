//! Kaniran sidecars — Rust-only runtime engines that interpret the
//! data-row registries the upstream macros write to. No Lisp FQNs.
//!
//! Folds three per-symbol sidecars into one module:
//!
//! - **counter-args** ([`CounterArgs`] / [`CounterClass`] +
//!   [`args`] / [`args_multi`] / [`args_suffix`] / [`digit_opts`]) —
//!   the keyword-arg recipe that `def-special-counter` callsites
//!   produce; the `*counter-cache*` populator stores one per text key,
//!   and `find-counter` later applies it to `make-instance` to
//!   construct a [`crate::dict::counters::classes::Counter`]. Diverges
//!   from upstream by pre-expanding multi-text in [`args_multi`] rather
//!   than deferring to the populator's `add-args`; output is identical
//!   given correct insertion order.
//!
//! - **hint-engine** ([`run_easy_hint`] / [`finish_simple_hint`] +
//!   helpers) — the interpreter for the bodies that `def-easy-hint`
//!   (`dict-split.lisp:916`) and `def-simple-hint`
//!   (`dict-split.lisp:860`) expand into. The macros each yield the
//!   same shape per callsite (type-check `simple-text`, compute
//!   `match-diff` / `match-readings`, translate hint positions, splice
//!   sentinels via [`crate::dict::split::hint::insert_hints`]); this
//!   engine holds the shared helpers, and the per-callsite data lives
//!   in [`crate::dict::split::hint_map`].
//!
//! - **split-engine** ([`run_split`] + [`SplitDef`] / [`Step`] /
//!   [`Pred`] / [`Modify`] / etc.) — the interpreter for every
//!   `def-simple-split` callsite (`dict-split.lisp:13`) and its
//!   derived helpers (`def-de-split`, `def-toori-split`, `def-do-split`,
//!   `def-shi-split`). Each macro expands every callsite into the same
//!   `prog*`-skeleton template; the engine factors that template out so
//!   each registered split is a single [`SplitDef`] data row in
//!   [`crate::dict::split::split_map`] / [`crate::dict::split::segsplit`].

use std::collections::HashMap;
use std::sync::OnceLock;

use super::word::{
    KaniHintKind, KaniMatchPart, KaniSimpleTextDispatchEnum, KaniWordDispatchEnum, SplitPart,
    SuffixKind,
};
use crate::characters::text_utils::{match_diff, safe_subseq, MatchSegment};
use crate::characters::voicing::unrendaku as unrendaku_fn;
use crate::conn::kani_context::KaniranContext;
use crate::dict::counters::classes::{
    Common, CounterSource, DigitOp, DigitOptEntry, DigitOptKey,
};
use crate::dict::find_word_conj_of::find_word_conj_of;
use crate::dict::find_word_seq::find_word_seq;
use crate::dict::get_kana::get_kana;
use crate::dict::split::hint::insert_hints;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::split::split::optprefix;
use crate::dict::split::hint::translate_hints;
use crate::dict::true_kana::true_kana;
use crate::dict::true_kanji::true_kanji;
use crate::dict::word_type::WordType;
use crate::kanji::matching::{match_readings, MatchedSegment};

// =========================================================================
// CounterArgs (was kani_counter_args.rs)
// =========================================================================

/// Tag-only twin of [`crate::dict::counters::classes::Counter`].
/// Separate so [`CounterArgs`] can stay `Clone` without forcing it
/// onto every variant struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterClass {
    Text,
    NumberText,
    Halfhour,
    Hifumi,
    Tsu,
    Wari,
    Age,
    DaysKun,
    DaysOn,
    Months,
    People,
}

#[derive(Debug, Clone)]
pub struct CounterArgs {
    pub class: CounterClass,
    pub text: String,
    pub kana: String,
    pub source: Option<CounterSource>,
    pub digit_opts: Vec<DigitOptEntry>,
    pub digit_set: Vec<i32>,
    pub allowed: Vec<i32>,
    pub foreign: bool,
    pub common: Common,
    pub accepts: Vec<SuffixKind>,
    pub suffix_descriptions: Vec<String>,
    pub ordinalp: bool,
    pub suffix: Option<String>,
}

impl CounterArgs {
    pub fn new(class: CounterClass, text: impl Into<String>, kana: impl Into<String>) -> Self {
        CounterArgs {
            class,
            text: text.into(),
            kana: kana.into(),
            source: None,
            digit_opts: Vec::new(),
            digit_set: Vec::new(),
            allowed: Vec::new(),
            foreign: false,
            common: Common::Inherit,
            accepts: Vec::new(),
            suffix_descriptions: Vec::new(),
            ordinalp: false,
            suffix: None,
        }
    }

    pub fn source(mut self, source: Option<CounterSource>) -> Self {
        self.source = source;
        self
    }

    pub fn digit_opts(mut self, opts: Vec<DigitOptEntry>) -> Self {
        self.digit_opts = opts;
        self
    }

    pub fn digit_set(mut self, set: Vec<i32>) -> Self {
        self.digit_set = set;
        self
    }

    pub fn allowed(mut self, allowed: Vec<i32>) -> Self {
        self.allowed = allowed;
        self
    }

    pub fn foreign(mut self, foreign: bool) -> Self {
        self.foreign = foreign;
        self
    }

    pub fn common(mut self, common: Common) -> Self {
        self.common = common;
        self
    }

    pub fn accepts(mut self, accepts: Vec<SuffixKind>) -> Self {
        self.accepts = accepts;
        self
    }

    pub fn suffix_descriptions(mut self, descriptions: Vec<String>) -> Self {
        self.suffix_descriptions = descriptions;
        self
    }

    pub fn ordinalp(mut self, ordinalp: bool) -> Self {
        self.ordinalp = ordinalp;
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }
}

/// Mirrors the Lisp `(find query readings :key 'text :test 'equal)`.
pub fn find_source(query: &str, kanji: &[KanjiText], kana: &[KanaText]) -> Option<CounterSource> {
    if let Some(r) = kanji.iter().find(|r| r.text == query) {
        return Some(CounterSource::Kanji(r.clone()));
    }
    if let Some(r) = kana.iter().find(|r| r.text == query) {
        return Some(CounterSource::Kana(r.clone()));
    }
    None
}

/// Single-text entry. Mirrors `(args class text kana ...)`.
pub fn args(
    class: CounterClass,
    text: &str,
    kana: &str,
    kanji: &[KanjiText],
    kana_rows: &[KanaText],
) -> CounterArgs {
    CounterArgs::new(class, text, kana).source(find_source(text, kanji, kana_rows))
}

/// Mirrors `(args class '(t1 t2 ...) kana ...)`. Eager per-text
/// expansion — see module doc for the upstream divergence.
pub fn args_multi(
    class: CounterClass,
    texts: &[&str],
    kana: &str,
    kanji: &[KanjiText],
    kana_rows: &[KanaText],
) -> Vec<CounterArgs> {
    texts
        .iter()
        .map(|t| CounterArgs::new(class, *t, kana).source(find_source(t, kanji, kana_rows)))
        .collect()
}

/// Mirrors `(args-suffix class '(stem suf) '(kana-stem kana-suf) ...)`.
/// Cache key = stem + suf concatenated; `:kana` = stem kana; `:suffix`
/// = suf kana; `:source` = stem's row.
pub fn args_suffix(
    class: CounterClass,
    text_parts: (&str, &str),
    kana_parts: (&str, &str),
    kanji: &[KanjiText],
    kana_rows: &[KanaText],
) -> CounterArgs {
    let (stem, suf) = text_parts;
    let (kana_stem, kana_suf) = kana_parts;
    let combined = format!("{}{}", stem, suf);
    CounterArgs::new(class, combined, kana_stem)
        .suffix(kana_suf)
        .source(find_source(stem, kanji, kana_rows))
}

/// Mirrors the Lisp shape `'((3 :r) (4 :h "よ"))` at the callsite.
pub fn digit_opts(items: &[(DigitOptKey, &[DigitOp])]) -> Vec<DigitOptEntry> {
    items
        .iter()
        .map(|(k, ops)| DigitOptEntry { key: *k, ops: ops.to_vec() })
        .collect()
}

// =========================================================================
// Hint engine (was kani_hint_engine.rs)
// =========================================================================

/// Search for a hiragana substring inside a kana string, returning
/// the start char-position. `from_end = true` mirrors CL's
/// `(search needle haystack :from-end t)` — last occurrence.
pub fn search_chars(needle: &str, haystack: &str, from_end: bool) -> Option<usize> {
    let h: Vec<char> = haystack.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    if n.is_empty() {
        return Some(if from_end { h.len() } else { 0 });
    }
    if n.len() > h.len() {
        return None;
    }
    let last_start = h.len() - n.len();
    if from_end {
        (0..=last_start).rev().find(|&i| h[i..i + n.len()] == n[..])
    } else {
        (0..=last_start).find(|&i| h[i..i + n.len()] == n[..])
    }
}

/// `(alexandria:ends-with #\<c> k)` — true when `k`'s last char
/// equals `c`. Goes through `chars().last()` so the comparison
/// honors character (not byte) semantics.
pub fn ends_with_char(s: &str, c: char) -> bool {
    s.chars().last() == Some(c)
}

/// `def-simple-hint` body shared helper: build `(KaniHintKind,
/// usize)` from a signed position, dropping the entry if the
/// position is negative. Mirrors upstream `insert-hints`'s
/// `(<= 0 position len)` guard absorbing negative positions
/// silently — `(- l 1)` with `l = 0` evaluates to -1 in CL,
/// reaches `insert-hints`, and gets dropped.
pub fn safe_hint(kind: KaniHintKind, pos: i64) -> Option<(KaniHintKind, usize)> {
    if pos >= 0 {
        Some((kind, pos as usize))
    } else {
        None
    }
}

/// Common tail for every `def-simple-hint` body: `(insert-hints
/// (get-kana ,reading-var) (list ,@hints-emits))`. The recursive
/// `get_kana` call observes the `:around` rebind via `ctx.disable_hints`
/// — the outer `simple-text :around` rebinds the ctx via
/// [`crate::conn::kani_context::KaniranContext::with_disable_hints`]`(true)`
/// before this body runs, and the rebound ctx threads down to here, so
/// the inner `:around` skips its hint branch. `Box::pin` breaks the
/// static-recursion cycle through get_kana ↔ get_hint ↔
/// hint_map_dispatch.
pub async fn finish_simple_hint(
    ctx: &KaniranContext,
    reading: &KaniWordDispatchEnum,
    hints: Vec<(KaniHintKind, usize)>,
) -> Result<Option<String>, sqlx::Error> {
    // get_kana returning None means upstream `(text nil)` —
    // no kana representation exists for this reading. The
    // hint body's `(insert-hints (get-kana reading) ...)`
    // would crash upstream; mirror as no-hint here.
    let Some(kana) = Box::pin(get_kana(ctx, reading)).await? else {
        return Ok(None);
    };
    Ok(Some(insert_hints(&kana, &hints)))
}

/// Compute the `kana-var` once at the top of each simple-hint body,
/// matching the macro's `(let* ((,kana-var (true-kana ...))
/// (,length-var (length ,kana-var)) ...))` prologue. Returns
/// `Ok(None)` when `true_kana` would surface upstream's `(text nil)`
/// crash — caller treats as a no-hint result.
pub async fn true_kana_and_len(
    ctx: &KaniranContext,
    reading: &KaniWordDispatchEnum,
) -> Result<Option<(String, i64)>, sqlx::Error> {
    let Some(k) = true_kana(ctx, reading).await? else {
        return Ok(None);
    };
    let l = k.chars().count() as i64;
    Ok(Some((k, l)))
}

/// `kanji_split` is the literal string passed to `def-easy-hint`
/// (e.g. `"郷 に 入って は 郷 に 従え"`). Space-separated parts;
/// every interior space marks a hint position.
#[derive(Debug, Clone, Copy)]
pub struct EasyHint {
    pub seq: i32,
    pub kanji_split: &'static str,
}

/// Pre-computed `(text, hints)` for one [`EasyHint`] — the upstream
/// macroexpansion of `def-easy-hint` produces these as static
/// literals at compile time. The Rust port computes them on first
/// dispatch for the entry's seq and caches keyed by `seq` via
/// [`parsed_easy_hint`].
struct ParsedEasyHint {
    text: String,
    hints: Vec<(KaniHintKind, usize)>,
}

/// Lookup-or-compute the parsed `(text, hints)` for `hint`. Caches
/// keyed by `seq` so each `def-easy-hint` callsite incurs the
/// `parse_kanji_split` scan once per process lifetime — mirroring the
/// upstream's macroexpand-time pre-compute.
fn parsed_easy_hint(hint: &EasyHint) -> &'static ParsedEasyHint {
    static CACHE: OnceLock<HashMap<i32, ParsedEasyHint>> = OnceLock::new();
    let map = CACHE.get_or_init(|| {
        crate::dict::split::hint_map::EASY_HINTS
            .iter()
            .map(|e| {
                let (text, hints) = parse_kanji_split(e.kanji_split);
                (e.seq, ParsedEasyHint { text, hints })
            })
            .collect()
    });
    map.get(&hint.seq).expect("EasyHint seq not in EASY_HINTS table")
}

/// Run the body that `def-easy-hint` expands into. Returns
/// `Ok(None)` when the reading isn't a `simple-text`, when the
/// alignment fails (`match_diff` or `match_readings` return
/// `None`), or when `get_kana` runs but every translated hint is
/// out-of-range (the result of `insert_hints` with an empty hints
/// list is the unhinted kana; we still wrap in `Some` to mirror
/// the upstream `(insert-hints (get-kana reading) ...)` returning
/// the kana string).
pub async fn run_easy_hint(
    ctx: &KaniranContext,
    hint: &EasyHint,
    reading: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    // dict-split.lisp:931 — (when (typep ,reading-var 'simple-text))
    match reading {
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Proxy(_) => {}
        _ => return Ok(None),
    }

    // The macro pre-computes ,text (kanji-split with spaces removed)
    // and ,hints (the list of (kw pos) emits) at compile time. Rust
    // mirrors that via a OnceLock cache populated on first dispatch
    // (see `parsed_easy_hint` / module-level CACHE).
    let parsed = parsed_easy_hint(hint);

    // dict-split.lisp:932 — (rtext = (true-kanji reading))
    let Some(rtext) = true_kanji(ctx, reading).await? else {
        return Ok(None);
    };

    // dict-split.lisp:933 — (match = (match-diff text rtext))
    let Some((md, _md_score)) = match_diff(&parsed.text, &rtext) else {
        return Ok(None);
    };
    let md_parts: Vec<KaniMatchPart> = md
        .iter()
        .map(|seg| match seg {
            MatchSegment::Equal(s) => KaniMatchPart::Atom(s.chars().count()),
            MatchSegment::Diff(a, b) => {
                KaniMatchPart::Pair(a.chars().count(), b.chars().count())
            }
        })
        .collect();

    // dict-split.lisp:934 — (kr = (match-readings rtext (true-kana reading)))
    let Some(tk) = true_kana(ctx, reading).await? else {
        return Ok(None);
    };
    let Some(kr) = match_readings(ctx, &rtext, &tk).await? else {
        return Ok(None);
    };
    let kr_parts: Vec<KaniMatchPart> = kr
        .iter()
        .map(|seg| match seg {
            MatchedSegment::NonKanji(s) => KaniMatchPart::Atom(s.chars().count()),
            MatchedSegment::Kanji { kanji, reading } => KaniMatchPart::Pair(
                kanji.chars().count(),
                reading.reading.chars().count(),
            ),
        })
        .collect();

    // dict-split.lisp:936 — (translate-hints kr (translate-hints match hints))
    let translated1 = translate_hints(&md_parts, &parsed.hints);
    let translated2 = translate_hints(&kr_parts, &translated1);

    // dict-split.lisp:936 — (insert-hints (get-kana reading) ...).
    // Box::pin breaks the static-recursion cycle through
    // get_kana → get_hint → hint_map_dispatch → run_easy_hint → get_kana.
    // The recursion is bounded at runtime by `ctx.disable_hints =
    // true` (the outer get_kana :around rebinds the ctx via
    // [`KaniranContext::with_disable_hints`] before calling get_hint;
    // hint_map_dispatch threads the rebound ctx down to this fn, so
    // the inner get_kana reads true and skips the hint branch). None
    // propagates from upstream's `(insert-hints nil ...)` no-kana
    // case.
    let Some(kana) = Box::pin(get_kana(ctx, reading)).await? else {
        return Ok(None);
    };
    Ok(Some(insert_hints(&kana, &translated2)))
}

/// Compile-time / load-time computation that `def-easy-hint`
/// performs at macroexpand:
///
/// ```lisp
/// (parts (split-sequence #\Space kanji-split))
/// (text (remove #\Space kanji-split))
/// (hints (loop with pos = 0
///              for part in parts
///              unless (zerop pos)
///              collect (list :space pos)
///              and if (find part '("は" "へ" "には" "とは") :test 'equal)
///              collect (list :mod (+ pos (length part) -1))
///              do (incf pos (length part))))
/// ```
///
/// Positions are character offsets (the upstream uses `(length
/// part)` on Lisp strings, which is char-count in SBCL).
fn parse_kanji_split(kanji_split: &str) -> (String, Vec<(KaniHintKind, usize)>) {
    let mut text = String::with_capacity(kanji_split.len());
    let mut hints: Vec<(KaniHintKind, usize)> = Vec::new();
    let mut pos: usize = 0;
    for (i, part) in kanji_split.split(' ').enumerate() {
        // dict-split.lisp:921 — `unless (zerop pos)` gates BOTH the
        // :space emit and the `and if`-joined :mod emit. A :mod
        // would otherwise fire even when the part appears at index 0
        // (kanji_split starting with one of the trigger strings),
        // which the upstream `unless` deliberately suppresses.
        if i > 0 {
            // dict-split.lisp:922 — (collect (list :space pos))
            hints.push((KaniHintKind::Space, pos));
            if matches!(part, "は" | "へ" | "には" | "とは") {
                // dict-split.lisp:923-924 —
                // (collect (list :mod (+ pos (length part) -1)))
                let part_len = part.chars().count();
                hints.push((KaniHintKind::Mod, pos + part_len - 1));
            }
        }
        text.push_str(part);
        pos += part.chars().count();
    }
    (text, hints)
}

// =========================================================================
// Split engine (was kani_split_engine.rs)
// =========================================================================

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

// =========================================================================
// Tests (was kani_hint_engine.rs::tests)
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// "郷 に 入って は 郷 に 従え" — parts = ["郷", "に", "入って",
    /// "は", "郷", "に", "従え"]. Joined text = "郷に入っては郷に従え"
    /// (10 chars). Hints fire at every interior space (pos=1,2,5,6,7,8)
    /// plus the `は`-mod at pos `5 + 1 - 1 = 5` (は starts at pos 5
    /// with part_len = 1).
    #[test]
    fn parse_typical_easy_hint() {
        let (text, hints) = parse_kanji_split("郷 に 入って は 郷 に 従え");
        assert_eq!(text, "郷に入っては郷に従え");
        assert_eq!(
            hints,
            vec![
                (KaniHintKind::Space, 1), // before に
                (KaniHintKind::Space, 2), // before 入って
                (KaniHintKind::Space, 5), // before は
                (KaniHintKind::Mod, 5),   // は's mod
                (KaniHintKind::Space, 6), // before 郷
                (KaniHintKind::Space, 7), // before に
                (KaniHintKind::Space, 8), // before 従え
            ]
        );
    }

    /// "とは" appears in trigger set — emits :mod at pos + len - 1
    /// when starting at offset 0.
    #[test]
    fn parse_with_toha_emits_mod() {
        let (text, hints) = parse_kanji_split("とは 言うものの");
        assert_eq!(text, "とは言うものの");
        assert_eq!(
            hints,
            vec![(KaniHintKind::Space, 2),]
        );
        // ↑ no :mod for "とは" at index 0 because the macro's
        // `unless (zerop pos)` gates BOTH the space and the mod emit
        // (the `and if` clause runs only when the unless succeeds).
    }

    /// Single-part: no interior space, no hints.
    #[test]
    fn parse_single_part_emits_no_hints() {
        let (text, hints) = parse_kanji_split("おはよう");
        assert_eq!(text, "おはよう");
        assert!(hints.is_empty());
    }

    /// "は" inside emits a :mod at pos + len - 1 = pos (len("は")=1).
    /// Verified against the upstream macroexpansion of
    /// `(def-easy-hint 1338260 "出る 釘 は 打たれる")` (REPL).
    #[test]
    fn parse_ha_in_middle() {
        let (text, hints) = parse_kanji_split("出る 釘 は 打たれる");
        assert_eq!(text, "出る釘は打たれる");
        // parts: 出る(2) 釘(1) は(1) 打たれる(4); pos values at each
        // interior boundary: 2 (before 釘), 3 (before は), 4 (before
        // 打たれる). The は part also emits :mod at pos=3.
        assert_eq!(
            hints,
            vec![
                (KaniHintKind::Space, 2),
                (KaniHintKind::Space, 3),
                (KaniHintKind::Mod, 3),
                (KaniHintKind::Space, 4),
            ]
        );
    }
}
