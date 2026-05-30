//! Port of the dict-split.lisp hint-logic layer — the runtime
//! helpers that splice / project / strip hint sentinels around the
//! `*hint-map*` dispatcher (see [`super::hint_map`]), the top-level
//! `get-hint` lookup driven by `(seq reading)`, and the `*hints-
//! checked*` / `*easy-hints-seqs*` audit-only globals plus their sole
//! consumer `check-easy-hints`.
//!
//! Section order mirrors upstream's `dict-split.lisp:826-955`:
//! process-hints (826), strip-hints (829), insert-hints (834),
//! translate-hint-position (882), translate-hints (899),
//! `*easy-hints-seqs*` (904), check-easy-hints (906),
//! `*hints-checked*` (947), get-hint (938).
//!
//! The audit-only artifacts (`*easy-hints-seqs*`, check-easy-hints,
//! `*hints-checked*`) are gated under `#[cfg(test)]` so they're absent
//! from release binaries, matching the upstream "Only used for testing"
//! docstring at `dict-split.lisp:904`.

use crate::characters::normalize::simplify_ngrams;
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data::conj_data_from;
use crate::dict::kani::{KaniHintKind, KaniMatchPart, KaniWordDispatchEnum};
use crate::dict::counters::dispatchers::seq as word_seq;
use crate::dict::split::hint_map::{
    hint_map_dispatch, hint_simplify_map, HintDispatch, HINT_CHAR_MAP,
};
use crate::dict::word_info::word_conj_data;
use crate::dict::word_info::WordInfoSeq;

// =========================================================================
// process-hints (dict-split.lisp:826)
// =========================================================================

/// Applies the hint-substitution table to a romanizer-ready kana
/// string: collapses each `(*kana-hint-mod* + は|ハ|へ|ヘ)` digram
/// into its rewritten reading and converts standalone sentinels
/// back to user-visible characters (or drops them). See
/// [`hint_simplify_map`] for the substitution rules.
pub fn process_hints(word: &str) -> String {
    simplify_ngrams(word, hint_simplify_map())
}

// =========================================================================
// strip-hints (dict-split.lisp:829)
// =========================================================================

/// Drops every occurrence of a hint sentinel character (i.e. the
/// values held in [`HINT_CHAR_MAP`]) from `word`. The Lisp
/// `(remove-if (lambda (c) (find c *hint-char-map*)) word)` treats
/// the plist as a flat sequence and tests `c` against every element;
/// only the value side ever collides because the keys are symbols,
/// not characters.
pub fn strip_hints(word: &str) -> String {
    word.chars()
        .filter(|c| !HINT_CHAR_MAP.iter().any(|(_, hc)| hc == c))
        .collect()
}

// =========================================================================
// insert-hints (dict-split.lisp:834)
// =========================================================================

/// Splice hint sentinels into a kana string at the positions named
/// by `hints`. Each `(kind, pos)` entry pushes the character looked
/// up via [`HINT_CHAR_MAP`] into a bucket at character index `pos`
/// (where `pos` may equal the string's character length, meaning
/// "after the last char"). Hints whose position exceeds the string
/// length are silently dropped — the Lisp guard `(<= 0 position len)`
/// covers both polarities; in Rust the lower bound is enforced by the
/// `usize` type and the upper bound is checked explicitly.
///
/// When two or more hints land at the same position, the resulting
/// sentinels are emitted in the order the hints were supplied
/// (mirroring the upstream `push` + `reverse` pair). Returns the
/// input unchanged when `hints` is empty.
pub fn insert_hints(s: &str, hints: &[(KaniHintKind, usize)]) -> String {
    if hints.is_empty() {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut positions: Vec<Vec<char>> = vec![Vec::new(); len + 1];
    for (kind, pos) in hints {
        if *pos > len {
            continue;
        }
        // dict-split.lisp:840 (getf *hint-char-map* character-kw)
        let ch = HINT_CHAR_MAP
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, c)| *c)
            .expect("HINT_CHAR_MAP covers every KaniHintKind variant");
        positions[*pos].push(ch);
    }
    let mut out = String::with_capacity(s.len() + hints.len() * 3);
    for i in 0..=len {
        for ch in &positions[i] {
            out.push(*ch);
        }
        if i < len {
            out.push(chars[i]);
        }
    }
    out
}

// =========================================================================
// translate-hint-position (dict-split.lisp:882)
// =========================================================================

/// Walks `matched` — a heterogeneous alignment list whose elements
/// are either an equal substring or a `(pre, post)` differing pair —
/// and translates a character index `position` over the pre-image
/// axis into a character index on the post-image axis. Returns
/// `None` when `position` overshoots the alignment's total
/// pre-image length (the upstream `loop` falls through and returns
/// `nil`).
///
/// The two diff-pair sub-cases mirror the upstream `cond` exactly:
/// - `position` strictly inside a pair: snap to `off + min(1,
///   max(clen, rem))` — i.e. the start of the post-image segment
///   when either side is non-empty, or `off` when both are empty.
/// - `position` at the trailing edge of a pair: snap to `off +
///   clen` (the end of the post-image segment).
///
/// The Lisp consumes the raw output of `match-diff` /
/// `match-readings` and dispatches by `(if (atom part) ...)`. The
/// Rust port consumes [`KaniMatchPart`], an explicit two-variant
/// enum carrying pre-computed character lengths — the function only
/// ever calls `length` on the substrings, so the substrings
/// themselves are dropped at conversion time. Observable behavior
/// is identical.
pub fn translate_hint_position(matched: &[KaniMatchPart], position: usize) -> Option<usize> {
    let mut off: usize = 0;
    let mut rem: usize = position;
    for part in matched {
        match part {
            KaniMatchPart::Atom(len) => {
                if rem <= *len {
                    return Some(off + rem);
                }
                rem -= *len;
                off += *len;
            }
            KaniMatchPart::Pair(len, clen) => {
                if rem < *len {
                    return Some(off + 1usize.min(rem.max(*clen)));
                }
                if rem == *len {
                    return Some(off + *clen);
                }
                rem -= *len;
                off += *clen;
            }
        }
    }
    None
}

// =========================================================================
// translate-hints (dict-split.lisp:899)
// =========================================================================

/// Re-projects every `(kind, pos)` entry in `hints` through the
/// alignment `matched`. Entries whose position overshoots the
/// alignment (where [`translate_hint_position`] returns `None`)
/// drop out of the result — the upstream `if new-pos collect` only
/// collects on non-nil.
pub fn translate_hints(
    matched: &[KaniMatchPart],
    hints: &[(KaniHintKind, usize)],
) -> Vec<(KaniHintKind, usize)> {
    hints
        .iter()
        .filter_map(|(kind, pos)| {
            translate_hint_position(matched, *pos).map(|new_pos| (*kind, new_pos))
        })
        .collect()
}

// =========================================================================
// get-hint (dict-split.lisp:938)
// =========================================================================

/// Look up a hint function for `reading` in [`super::hint_map`],
/// applying the first one that matches:
///
/// 1. `(gethash (seq reading) *hint-map*)` — direct lookup on the
///    reading's own seq. If a registered hint-fn exists, call it.
/// 2. Otherwise walk every `from`-seq in `(mapcar #'conj-data-from
///    (word-conj-data reading))`, returning the first hint result
///    that fires.
///
/// Returns `None` when neither path produces a hint — meaning the
/// `get-kana :around` method on `simple-text` falls through to
/// `(call-next-method)`.
///
/// ## Divergences
///
/// Diverges from the upstream lambda list `(reading)` by:
/// - taking `&KaniranContext` for the database handle (transitively,
///   via [`word_conj_data`] and the hint-fn bodies), replacing the
///   upstream dynamic `*connection*` per
///   [`crate::conn::kani_context`]. The ctx also carries the
///   `*disable-hints*` binding (see
///   [`crate::dict::best_text::get_kana`] for the rationale);
///   callers invoke `get_hint` after rebinding via
///   [`crate::conn::kani_context::KaniranContext::with_disable_hints`]`(true)`.
/// - returning `Result<Option<String>, sqlx::Error>` — hint-fn bodies
///   reach the database (via [`crate::dict::best_text::true_kana`] /
///   [`crate::dict::best_text::true_kanji`] /
///   [`crate::kanji::matching::match_readings`]), so errors propagate;
/// - the hint-fn invocation itself dispatches through
///   [`hint_map_dispatch`] (the static table replacing the upstream
///   runtime hashtable per CONVENTIONS §5.4 + the `*split-map*`
///   precedent).
///
/// ## Upstream gf cross-dispatch
///
/// Upstream `(seq reading)` is itself a generic function with five
/// method bodies (proxy / compound / counter-text / simple-text
/// slot reader / entry). Since `get-hint` is called from the
/// `simple-text :around` method on `get-kana` (`dict.lisp:80-84`),
/// the receiver here is always a `kanji-text`, `kana-text`, or
/// `proxy-text`. For proxy-text, `(seq proxy)` is
/// `(seq (source proxy))`, which the Rust dispatcher routes through
/// [`crate::dict::counters::dispatchers::seq`].
pub async fn get_hint(
    ctx: &KaniranContext,
    reading: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    // dict-split.lisp:939 — (gethash (seq reading) *hint-map*)
    let primary_seq = match word_seq(reading) {
        Some(WordInfoSeq::Single(s)) => Some(s),
        // compound-text returns a list; get-hint's hashtable keys are
        // single ints. Upstream `(gethash <list> *hint-map*)` always
        // misses (lists don't hash to integers), so treat as no
        // primary lookup. (Get-kana :around only fires for simple-text
        // anyway — this branch is defensive.)
        Some(WordInfoSeq::Multi(_)) | None => None,
    };
    // dict-split.lisp:941-942 — `(if hint-fn (funcall hint-fn reading) ...)`.
    // When the primary seq IS registered, return its body's result
    // directly — even if the body returned nil (a `:test` clause
    // failed). The conj-of walk only fires for an UNREGISTERED
    // primary, not for a registered-but-nil-returning one.
    if let Some(s) = primary_seq {
        match hint_map_dispatch(ctx, s, reading).await? {
            HintDispatch::Registered(result) => return Ok(result),
            HintDispatch::Unregistered => { /* fall through to conj-of walk */ }
        }
    }

    // dict-split.lisp:943-945 — walk conj-of seqs. The upstream
    // `when hint-fn do (return (funcall hint-fn reading))` returns
    // the funcall result on the FIRST registered seq, whatever the
    // body returned (Some or None). Subsequent conj-of seqs are
    // never tried after the first hit, even if its body returned nil.
    let conj_data = word_conj_data(ctx, reading).await?;
    for cd in &conj_data {
        if let Some(from_seq) = conj_data_from(cd) {
            match hint_map_dispatch(ctx, from_seq, reading).await? {
                HintDispatch::Registered(result) => return Ok(result),
                HintDispatch::Unregistered => continue,
            }
        }
    }
    Ok(None)
}

// =========================================================================
// *hints-checked* (dict-split.lisp:947) — audit-only
// =========================================================================

/// Upstream construction is `(mapcar 'car '(<alist>))` where each
/// alist entry pairs a seq with a sample expression for the audit
/// comment; the port inlines the post-`mapcar` list of cars.
/// Duplicates (e.g. `2006850` appearing three times) are preserved
/// because the upstream consumer treats it as a list, not a set.
///
pub static HINTS_CHECKED: &[i32] = &[
    1186700, 1236510, 1252080, 1259320, 1259320, 1324680, 1327220, 1348240, 1370020, 1483810,
    1531720, 1535270, 1540770, 1632820, 1636580, 1641640, 1671190, 1856780, 1872190, 1872750,
    1899360, 1901660, 1917360, 2006850, 2006850, 2006850, 2020910, 2029360, 2067580, 2067580,
    2095060, 2099720, 2099720, 2099770, 2101090, 2114550, 2115810, 2121160, 2125840, 2140480,
    2183830, 2183840, 2207940, 2215370, 2223210, 2263410, 2276210, 2276210, 2399360, 2399890,
    2401870, 2402670, 2407860, 2557390, 2560100, 2568460, 2603950, 2627910, 2655400, 2655420,
    2657160, 2678440, 2684060, 2684060, 2709160, 2717360, 2727860, 2729830, 2755410, 2759720,
    2776170, 2777440, 2793750, 2795820, 2799570, 2817370, 2826563, 2827090, 2829589, 2831138,
    2832146, 2832146, 2833092, 2833874, 2834024, 2835778, 2836685, 2836884, 2837561, 2837561,
    2837752, 2837752, 2839604, 2841916, 1002340, 2159030, 2131510, 2131510, 2131510, 2849623,
    2238150, 2832275, 2832275, 2850988, 1344300, 2102270, 2708470, 2770500, 2770500, 2788150,
    2859213, 2858410, 2860289, 1614040, 1639300, 2849161, 2859753, 2859754, 2859754, 2859776,
    2102020, 2102630, 2102630, 2239210, 2862603, 2863695, 2863695, 1858020, 1893350, 2213430,
    2628190, 2864291, 2868374, 1586300, 1185210, 1381650, 1381650, 1919400, 2217150, 2222890,
    2399430, 2399440, 2761770, 2761770, 2794610, 2796060, 2803060, 2803060, 2803060, 2830220,
    2830220, 2830575, 2418770, 2844727, 2844727, 2847931, 2848855, 1626200, 2126810, 2756140,
    2756140, 2864371,
];

// =========================================================================
// *easy-hints-seqs* (dict-split.lisp:904) — audit-only
// =========================================================================

/// Cumulative list of JMdict sequence ids registered by every
/// `def-easy-hint` form in `dict-split.lisp` (lines 1389-1859). The
/// upstream `defparameter` starts at `nil`; each `def-easy-hint`
/// call expands into a `(push ,seq *easy-hints-seqs*)` plus a
/// `(defhint (,seq) ...)` registration into
/// [`super::hint_map::EASY_HINTS`]. Since the same callsite populates
/// both globals, this port derives the seq list from
/// [`super::hint_map::EASY_HINTS`] via [`std::sync::OnceLock`] per
/// CONVENTIONS §5.2 — "build it from them, don't hand-copy."
///
/// Only consumer is [`check_easy_hints`] — a test-only sanity-scan.
/// Upstream marks the symbol "Only used for testing"
/// (`dict-split.lisp:904` docstring), so this fn is gated under
/// `#[cfg(test)]` and absent from release binaries.
///
/// Order matches the upstream `push` semantics (reverse of
/// source-file order). The check_easy_hints SQL `:in` filter
/// consumes the list as a set, so order doesn't affect behavior;
/// the test below pins the entry count.
#[cfg(test)]
pub fn easy_hints_seqs() -> &'static [i32] {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<i32>> = OnceLock::new();
    CACHE.get_or_init(|| {
        // EASY_HINTS is in source-file order (see hint_map).
        // Upstream `push` produces reverse-source order, so reverse here
        // to match the live SBCL image's *easy-hints-seqs* contents.
        super::hint_map::EASY_HINTS.iter().rev().map(|e| e.seq).collect()
    })
}

// =========================================================================
// check-easy-hints (dict-split.lisp:906) — audit-only
// =========================================================================

/// Test helper. For every kana-text row whose seq is registered in
/// [`easy_hints_seqs`] (one per `def-easy-hint` callsite), compute
/// `(true-kanji reading)` and `(true-kana reading)`, run them
/// through [`crate::kanji::matching::match_readings`], and collect
/// the readings whose `match-readings` returned no alignment. Used
/// upstream as a sanity-scan to surface easy-hint registrations
/// that won't fire because their kanji and kana don't align.
///
/// Upstream's only consumer is the test suite (the symbol's only
/// input is `*easy-hints-seqs*`, declared "Only used for testing"
/// at `dict-split.lisp:904`). The Rust port mirrors that — this fn
/// is gated under `#[cfg(test)]` and absent from release binaries.
///
/// Returns the rows that failed alignment as `(reading, kanji,
/// kana)` triples — same shape as the upstream
/// `(list reading kanji kana)` collected element.
///
/// ## Divergences
///
/// Diverges from the upstream lambda list `()` by:
/// - taking `&KaniranContext` for the database handle (replacing the
///   upstream `(with-db nil ...)` dynamic binding) per
///   [`crate::conn::kani_context`];
/// - returning `Vec<CheckEasyHintsFailure>` rather than a raw list of
///   3-element sub-lists. Each failure carries the unaltered
///   `KanaText` row plus the computed `kanji` (which can be `None`
///   when [`crate::dict::best_text::true_kanji`] returned `:null`)
///   and `kana` strings.
///
/// The upstream `(let ((*disable-hints* t)))` binding wraps the
/// entire loop body — `true-kanji`, `true-kana`, and
/// `match-readings` are all evaluated under the same rebind. The
/// Rust port mirrors that by rebinding the ctx once before the
/// loop via [`KaniranContext::with_disable_hints`]`(true)` and
/// passing `&ctx2` into all three calls (per the
/// [`crate::dict::best_text::get_kana`] divergence rationale:
/// ctx-slot beats task-local because the rebind survives rayon /
/// `tokio::spawn` boundaries the parallel pipeline introduces).
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct CheckEasyHintsFailure {
    pub reading: crate::dict::dao::KanaText,
    pub kanji: Option<String>,
    /// `None` mirrors upstream's `(text nil)` no-kana-row case
    /// (`get-kana` raises CL condition; Rust port surfaces None).
    /// A None `kana` is itself an alignment failure — recorded
    /// alongside true-kanji / true-kana misalignments.
    pub kana: Option<String>,
}

#[cfg(test)]
pub async fn check_easy_hints(
    ctx: &KaniranContext,
) -> Result<Vec<CheckEasyHintsFailure>, sqlx::Error> {
    use crate::dict::dao::KanaText;
    use crate::dict::best_text::true_kana;
    use crate::dict::best_text::true_kanji;
    use crate::kanji::matching::match_readings;
    // dict-split.lisp:908 — (select-dao 'kana-text (:in 'seq (:set *easy-hints-seqs*)))
    // Upstream uses a single `:in (:set ...)` clause. Postgres parameterized arrays
    // are equivalent — bind a single `&[i32]` and let sqlx generate the
    // `seq = ANY($1)` form. (sqlx::postgres doesn't expose IN-list directly.)
    let readings: Vec<KanaText> = sqlx::query_as(
        "SELECT * FROM kana_text WHERE seq = ANY($1)",
    )
    .bind(easy_hints_seqs())
    .fetch_all(&ctx.pool)
    .await?;

    // dict-split.lisp:909 — (let ((*disable-hints* t))) wraps the
    // entire loop body, covering true-kanji, true-kana, and
    // match-readings. Rebind the ctx once before the loop so all
    // three operations see the same binding (matches upstream
    // scope; today true-kanji + match-readings don't reach the
    // recursion guard so the binding is inert there, but the
    // scope must still match for any future code path that does).
    let ctx2 = ctx.with_disable_hints(true);
    let mut failures = Vec::new();
    for reading in readings {
        let lifted = KaniWordDispatchEnum::Kana(reading.clone());
        // dict-split.lisp:911 — (kanji = (true-kanji reading))
        let kanji = true_kanji(&ctx2, &lifted).await?;
        // dict-split.lisp:912 — (kana = (true-kana reading))
        let kana = true_kana(&ctx2, &lifted).await?;
        // dict-split.lisp:913-914 — (match = (match-readings kanji kana))
        // (unless match collect (list reading kanji kana))
        // Both `kanji` and `kana` can be None. Upstream behavior:
        // - `(match-readings nil kana)` returns nil (verified via
        //   kanji.lisp: `(make-rmap nil)` is nil, then
        //   `(match-readings* nil kana)` returns `:none`, outer
        //   `(unless (eql match :none) ...)` returns nil).
        // - `(match-readings kanji nil)` would raise on the inner
        //   `(length reading)` reading nil. The Rust None case for
        //   `kana` mirrors that by skipping the call and recording
        //   the misalignment.
        let match_result = match (&kanji, &kana) {
            (Some(k), Some(ka)) => match_readings(&ctx2, k, ka).await?,
            _ => None,
        };
        if match_result.is_none() {
            failures.push(CheckEasyHintsFailure {
                reading,
                kanji,
                kana,
            });
        }
    }
    Ok(failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::split::hint_map::{KANA_HINT_MOD, KANA_HINT_SPACE};

    // =====================================================================
    // translate_hint_position
    // =====================================================================

    /// Empty alignment: any position other than 0 overshoots and
    /// returns `None`. Position 0 also returns `None` because the
    /// `loop` never enters its body and the upstream returns `nil`.
    #[test]
    fn empty_alignment_returns_none() {
        assert_eq!(translate_hint_position(&[], 0), None);
        assert_eq!(translate_hint_position(&[], 5), None);
    }

    /// Single Atom: positions 0..=len map identity.
    #[test]
    fn atom_identity_map() {
        let m = [KaniMatchPart::Atom(3)];
        assert_eq!(translate_hint_position(&m, 0), Some(0));
        assert_eq!(translate_hint_position(&m, 1), Some(1));
        assert_eq!(translate_hint_position(&m, 3), Some(3));
        assert_eq!(translate_hint_position(&m, 4), None);
    }

    /// Pair snap: position strictly inside a pair returns `off + 1`
    /// when `clen >= 1`. (Upstream: (min 1 (max clen rem)) with
    /// rem >= 1 and clen >= 1 → 1.)
    #[test]
    fn pair_inside_snaps_to_one_when_post_nonempty() {
        let m = [KaniMatchPart::Pair(3, 2)];
        assert_eq!(translate_hint_position(&m, 1), Some(1));
        assert_eq!(translate_hint_position(&m, 2), Some(1));
    }

    /// Pair snap at trailing edge: position == len returns
    /// `off + clen` (end of post-image segment).
    #[test]
    fn pair_trailing_edge_returns_clen() {
        let m = [KaniMatchPart::Pair(3, 2)];
        assert_eq!(translate_hint_position(&m, 3), Some(2));
    }

    /// Both sides empty (`Pair(0, 0)`): inside-branch yields
    /// `off + 0`; the loop then advances by (0, 0) and continues.
    /// This is the same `< rem len` branch that fires when rem=0.
    #[test]
    fn pair_zero_zero_yields_off() {
        let m = [KaniMatchPart::Atom(2), KaniMatchPart::Pair(0, 0)];
        // rem=2 falls through atom, then rem=0 < len=0 is false,
        // rem=0 == len=0 hits the second branch, returns off+clen=2+0=2.
        assert_eq!(translate_hint_position(&m, 2), Some(2));
    }

    /// rem=0, Pair(0, 1): first branch `rem < len` is false (0 < 0
    /// is false). Second branch `rem == len` is true, returns
    /// off + clen = 0 + 1 = 1.
    #[test]
    fn pair_zero_post_one_at_position_zero() {
        let m = [KaniMatchPart::Pair(0, 1)];
        assert_eq!(translate_hint_position(&m, 0), Some(1));
    }

    /// rem=0, Pair(1, 0): first branch `rem < len` is true (0 < 1).
    /// max(clen=0, rem=0) = 0, min(1, 0) = 0. Returns off + 0.
    #[test]
    fn pair_one_post_zero_at_position_zero() {
        let m = [KaniMatchPart::Pair(1, 0)];
        assert_eq!(translate_hint_position(&m, 0), Some(0));
    }

    /// Multi-segment walk: Atom(2) + Pair(2, 3) + Atom(1).
    /// Total pre-length = 5; post-length = 6.
    #[test]
    fn multi_segment_walk() {
        let m = [
            KaniMatchPart::Atom(2),
            KaniMatchPart::Pair(2, 3),
            KaniMatchPart::Atom(1),
        ];
        // 0..=2 in the atom: identity
        assert_eq!(translate_hint_position(&m, 0), Some(0));
        assert_eq!(translate_hint_position(&m, 2), Some(2));
        // Pair pre-region 2..4: snap to atom-end + 1 = 3
        assert_eq!(translate_hint_position(&m, 3), Some(3));
        // Pair trailing edge: rem=2, len=2 → off + clen = 2 + 3 = 5
        assert_eq!(translate_hint_position(&m, 4), Some(5));
        // Trailing atom: at position=5, the pair "otherwise" branch
        // advances rem to 1 and off to 5; the Atom(1) returns
        // off+rem = 6 — the end of the entire post-image.
        assert_eq!(translate_hint_position(&m, 5), Some(6));
        // position=6 overshoots
        assert_eq!(translate_hint_position(&m, 6), None);
    }

    // =====================================================================
    // translate_hints
    // =====================================================================

    /// Empty hints → empty result.
    #[test]
    fn translate_empty_hints_empty_result() {
        let m = [KaniMatchPart::Atom(3)];
        assert!(translate_hints(&m, &[]).is_empty());
    }

    /// Empty alignment + non-empty hints → empty result (every
    /// hint overshoots).
    #[test]
    fn translate_empty_alignment_drops_all() {
        let hints = [(KaniHintKind::Mod, 1), (KaniHintKind::Space, 2)];
        assert!(translate_hints(&[], &hints).is_empty());
    }

    /// Identity walk over a pure Atom: every position passes
    /// through with the same index, hint kind preserved.
    #[test]
    fn translate_atom_identity_passthrough() {
        let m = [KaniMatchPart::Atom(5)];
        let hints = [(KaniHintKind::Mod, 1), (KaniHintKind::Space, 3)];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![(KaniHintKind::Mod, 1), (KaniHintKind::Space, 3)]
        );
    }

    /// Overshoot drops, in-range survives — output order matches
    /// the surviving subset of input order.
    #[test]
    fn translate_overshoot_filters_out() {
        let m = [KaniMatchPart::Atom(2)];
        let hints = [
            (KaniHintKind::Mod, 1),
            (KaniHintKind::Space, 5),
            (KaniHintKind::Mod, 2),
        ];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![(KaniHintKind::Mod, 1), (KaniHintKind::Mod, 2)]
        );
    }

    /// Walks a mixed Atom/Pair alignment — the position semantics
    /// follow [`translate_hint_position`] (verified there).
    #[test]
    fn translate_mixed_alignment_projection() {
        let m = [
            KaniMatchPart::Atom(2),
            KaniMatchPart::Pair(2, 3),
            KaniMatchPart::Atom(1),
        ];
        let hints = [
            (KaniHintKind::Mod, 1),   // atom-interior: 1
            (KaniHintKind::Space, 3), // pair-interior: 3
            (KaniHintKind::Mod, 4),   // pair-trailing: 5
        ];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![
                (KaniHintKind::Mod, 1),
                (KaniHintKind::Space, 3),
                (KaniHintKind::Mod, 5),
            ]
        );
    }

    // =====================================================================
    // insert_hints
    // =====================================================================

    /// Empty hints is a no-op.
    #[test]
    fn insert_empty_hints_returns_input() {
        assert_eq!(insert_hints("こんにちは", &[]), "こんにちは");
    }

    /// `:mod` at position `len-1` inserts the mod sentinel before
    /// the last character — mirrors the simple-hint rule
    /// `(:mod (- l 1))` for the `(2028920 ;; は)` group.
    #[test]
    fn insert_mod_before_last_char() {
        let out = insert_hints("こんにちは", &[(KaniHintKind::Mod, 4)]);
        assert_eq!(out, format!("こんにち{}は", KANA_HINT_MOD));
    }

    /// Position 0 prefixes the sentinel before the entire string.
    #[test]
    fn insert_position_zero_prefixes() {
        let out = insert_hints("は", &[(KaniHintKind::Mod, 0)]);
        assert_eq!(out, format!("{}は", KANA_HINT_MOD));
    }

    /// Position equal to length suffixes after the entire string.
    #[test]
    fn insert_position_equal_to_length_suffixes() {
        let out = insert_hints("ab", &[(KaniHintKind::Space, 2)]);
        assert_eq!(out, format!("ab{}", KANA_HINT_SPACE));
    }

    /// Out-of-range positions are silently dropped.
    #[test]
    fn insert_out_of_range_position_dropped() {
        assert_eq!(insert_hints("ab", &[(KaniHintKind::Mod, 5)]), "ab");
    }

    /// Multiple hints at the same position emit in the supplied
    /// order — verifies the `push` + `reverse` round-trip.
    #[test]
    fn insert_multiple_hints_same_position_keep_supplied_order() {
        let out = insert_hints(
            "ab",
            &[(KaniHintKind::Space, 1), (KaniHintKind::Mod, 1)],
        );
        assert_eq!(
            out,
            format!("a{}{}b", KANA_HINT_SPACE, KANA_HINT_MOD)
        );
    }

    /// Mixed `:space` + `:mod` at different positions — mirrors a
    /// `def-simple-hint` body with two emits.
    #[test]
    fn insert_mixed_kinds_at_different_positions() {
        let out = insert_hints(
            "ところへ",
            &[(KaniHintKind::Space, 3), (KaniHintKind::Mod, 3)],
        );
        assert_eq!(
            out,
            format!("ところ{}{}へ", KANA_HINT_SPACE, KANA_HINT_MOD)
        );
    }

    // =====================================================================
    // strip_hints
    // =====================================================================

    /// Round-trip with [`insert_hints`]: inserting a `:mod` sentinel
    /// then stripping yields the original kana.
    #[test]
    fn strip_strips_inserted_mod() {
        let with_hint = format!("こんにち{}は", KANA_HINT_MOD);
        assert_eq!(strip_hints(&with_hint), "こんにちは");
    }

    /// Strips both sentinels in one pass.
    #[test]
    fn strip_strips_space_and_mod() {
        let mixed = format!("a{}b{}c{}d", KANA_HINT_SPACE, KANA_HINT_MOD, KANA_HINT_SPACE);
        assert_eq!(strip_hints(&mixed), "abcd");
    }

    /// Regular ASCII space (U+0020) is not a sentinel and stays.
    #[test]
    fn strip_preserves_regular_space() {
        assert_eq!(strip_hints("a b c"), "a b c");
    }

    /// No sentinels — input passes through unchanged.
    #[test]
    fn strip_no_sentinels_unchanged() {
        assert_eq!(strip_hints("こんにちは"), "こんにちは");
    }

    /// Empty input.
    #[test]
    fn strip_empty_input() {
        assert_eq!(strip_hints(""), "");
    }

    // =====================================================================
    // process_hints
    // =====================================================================

    /// `*kana-hint-mod*` + `は` → `わ`. The canonical rewrite the
    /// hint system exists to enable.
    #[test]
    fn process_mod_plus_ha_becomes_wa() {
        let input = format!("こんにち{}は", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "こんにちわ");
    }

    /// `*kana-hint-mod*` + `へ` → `え`.
    #[test]
    fn process_mod_plus_he_becomes_e() {
        let input = format!("ところ{}へ", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "ところえ");
    }

    /// Katakana variants: `*kana-hint-mod*` + `ハ` → `ワ`,
    /// `*kana-hint-mod*` + `ヘ` → `エ`.
    #[test]
    fn process_katakana_variants() {
        let input = format!(
            "{m}ハ{m}ヘ",
            m = KANA_HINT_MOD,
        );
        assert_eq!(process_hints(&input), "ワエ");
    }

    /// `*kana-hint-space*` → ASCII space.
    #[test]
    fn process_space_sentinel_becomes_ascii_space() {
        let input = format!("ところ{}へ", KANA_HINT_SPACE);
        assert_eq!(process_hints(&input), "ところ へ");
    }

    /// Lone `*kana-hint-mod*` (no following は/ハ/へ/ヘ) drops.
    /// The order in `hint_simplify_map` ensures the 2-char rules
    /// fire first so we only fall through to the empty substitution
    /// when no digram matches.
    #[test]
    fn process_lone_mod_drops() {
        let input = format!("a{}b", KANA_HINT_MOD);
        assert_eq!(process_hints(&input), "ab");
    }

    /// No sentinels — pass-through.
    #[test]
    fn process_no_sentinels_unchanged() {
        assert_eq!(process_hints("こんにちは"), "こんにちは");
    }

    // =====================================================================
    // *easy-hints-seqs* / *hints-checked* count pins
    // =====================================================================

    /// Pins the entry count against the upstream observation. If a
    /// future upstream `dict-split.lisp` adds or removes a
    /// `def-easy-hint` form, this test fails until
    /// `super::hint_map::EASY_HINTS` is regenerated.
    #[test]
    fn easy_hints_seqs_entry_count_matches_upstream() {
        assert_eq!(easy_hints_seqs().len(), 431);
    }

    /// Pins the entry count against the live SBCL image
    /// (REPL probe 2026-05-17). A drift here means upstream
    /// `dict-split.lisp` added or removed audited-seq rows
    /// since this port; refresh the literal from a fresh
    /// `(length ichiran/dict::*hints-checked*)` probe.
    #[test]
    fn hints_checked_entry_count_matches_upstream() {
        assert_eq!(HINTS_CHECKED.len(), 162);
    }
}
