//! Rust-only sidecar (CONVENTIONS §1, §2): runtime engine for the
//! `def-easy-hint` and `def-simple-hint` callsites + shared
//! string-search / hint-emit helpers consumed by
//! [`super::_star_hint_map_star_::hint_map_dispatch`].
//!
//! No Lisp FQN. The macros `def-easy-hint` (`dict-split.lisp:916`)
//! and `def-simple-hint` (`dict-split.lisp:860`) each expand every
//! callsite into the same shape:
//!
//! - `def-easy-hint`: type-check `simple-text`, compute `match-diff`
//!   against the kanji-split + `match-readings` against the kana,
//!   translate the compile-time hint positions through both
//!   alignments, splice sentinels via [`super::insert_hints`].
//! - `def-simple-hint`: bind `(true-kana reading)` + its length,
//!   walk a per-callsite hints-def (with optional `:test` /
//!   `let*`-style position bindings), splice sentinels into
//!   `(get-kana reading)`.
//!
//! All the data-row registrations + the by-seq dispatch live in
//! [`super::_star_hint_map_star_`]; this file holds the engine
//! helpers shared across the bodies. Mirrors the
//! [`super::kani_split_engine`] pattern that absorbed
//! `def-simple-split`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::characters::text_utils::{match_diff, MatchSegment};
use crate::conn::kani_context::KaniranContext;
use crate::dict::get_kana::get_kana;
use crate::dict::insert_hints::insert_hints;
use crate::dict::kani_hint_kind::KaniHintKind;
use crate::dict::kani_match_part::KaniMatchPart;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::translate_hints::translate_hints;
use crate::dict::true_kana::true_kana;
use crate::dict::true_kanji::true_kanji;
use crate::kanji::matching::{match_readings, MatchedSegment};

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
        super::_star_hint_map_star_::EASY_HINTS
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
