use crate::conn::kani_backend::KaniBackend;
use crate::characters::char_class::simplify_ngrams;
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj::conj_data_from;
use crate::dict::counters::methods::seq as word_seq;
use crate::dict::dao::KanaText;
use crate::dict::kani_match_part::KaniMatchPart;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::split::hint_map::{hint_map_dispatch, HintDispatch, EASY_HINTS};
use crate::dict::split::kani_hint_kind::KaniHintKind;
use crate::dict::split::segsplit::{hint_simplify_map, HINT_CHAR_MAP};
use crate::dict::accessors::true_kana;
use crate::dict::accessors::true_kanji;
use crate::dict::accessors::word_conj_data;
use crate::dict::word_info::WordInfoSeq;
use crate::kanji::matching::match_readings;
use std::sync::OnceLock;

/// Port of `ichiran/dict:process-hints` (`dict-split.lisp:826-827`).
///
/// Applies the hint-substitution table to a romanizer-ready kana
/// string: collapses each `(*kana-hint-mod* + は|ハ|へ|ヘ)` digram into
/// its rewritten reading and converts standalone sentinels back to
/// user-visible characters (or drops them).
pub fn process_hints(word: &str) -> String {
    simplify_ngrams(word, hint_simplify_map())
}

/// Port of `ichiran/dict:strip-hints` (`dict-split.lisp:829-830`).
///
/// Drops every hint sentinel character (the values held in
/// [`crate::dict::split::segsplit::HINT_CHAR_MAP`]) from `word`.
pub fn strip_hints(word: &str) -> String {
    word.chars()
        .filter(|c| !HINT_CHAR_MAP.iter().any(|(_, hc)| hc == c))
        .collect()
}

/// Port of `ichiran/dict:insert-hints` (`dict-split.lisp:834-848`).
///
/// Splice hint sentinel characters into a kana string at the
/// character positions named by `hints`. A `pos` equal to the
/// string's char length means "after the last char"; hints whose
/// position exceeds the length are silently dropped. Multiple hints
/// at the same position are emitted in supplied order.
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

/// Port of `ichiran/dict:*easy-hints-seqs*` (`dict-split.lisp:904`).
///
/// List of JMdict sequence ids registered by every `def-easy-hint`
/// form. Upstream marks it "Only used for testing", so this module is
/// gated under `#[cfg(test)]` and absent from release binaries.
/// Derived from [`EASY_HINTS`] on first access. Mirrors the upstream
/// `(push ,seq *easy-hints-seqs*)` ordering — iteration is in the
/// reverse of source-file order, since `push` prepends.
pub fn easy_hints_seqs() -> &'static [i32] {
    static CACHE: OnceLock<Vec<i32>> = OnceLock::new();
    CACHE.get_or_init(|| {
        // EASY_HINTS is in source-file order (see _star_hint_map_star_).
        // Upstream `push` produces reverse-source order, so reverse here
        // to match the live SBCL image's *easy-hints-seqs* contents.
        EASY_HINTS.iter().rev().map(|e| e.seq).collect()
    })
}

/// Port of `ichiran/dict:translate-hint-position` (`dict-split.lisp:882-897`).
///
/// Translates a character index over an alignment's pre-image axis
/// into an index on the post-image axis, returning `None` when
/// `position` overshoots the alignment's total pre-image length.
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

/// Port of `ichiran/dict:translate-hints` (`dict-split.lisp:899-902`).
///
/// Re-projects every `(kind, pos)` entry in `hints` through the
/// alignment `matched`. Entries whose position overshoots the
/// alignment (where [`crate::dict::translate_hint_position`] returns
/// `None`) drop out of the result — the upstream `if new-pos
/// collect` only collects on non-nil.
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

/// Port of `ichiran/dict:*hints-checked*` (`dict-split.lisp:947`).
///
/// List of seqs whose split hints have been audited. Duplicates
/// (e.g. `2006850` three times) are preserved — it's a list, not a set.
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

/// Port of `ichiran/dict:check-easy-hints` (`dict-split.lisp:906-914`).
///
/// Test helper that scans every registered easy-hint kana row whose
/// kanji and kana readings fail to align under `match-readings`, and
/// returns those readings as `(reading, kanji, kana)` triples.
#[derive(Debug, Clone)]
pub struct CheckEasyHintsFailure {
    pub reading: KanaText,
    pub kanji: Option<String>,
    /// `None` mirrors upstream's `(text nil)` no-kana-row case
    /// (`get-kana` raises CL condition; Rust port surfaces None).
    /// A None `kana` is itself an alignment failure — recorded
    /// alongside true-kanji / true-kana misalignments.
    pub kana: Option<String>,
}

pub async fn check_easy_hints(
    ctx: &KaniranContext,
) -> Result<Vec<CheckEasyHintsFailure>, sqlx::Error> {
    // dict-split.lisp:908 — (select-dao 'kana-text (:in 'seq (:set *easy-hints-seqs*)))
    // Upstream uses a single `:in (:set ...)` clause. Postgres parameterized arrays
    // are equivalent — bind a single `&[i32]` and let sqlx generate the
    // `seq = ANY($1)` form. (sqlx::postgres doesn't expose IN-list directly.)
    let readings: Vec<KanaText> = ctx.store.kana_texts_by_seq_any(easy_hints_seqs()).await?;

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

/// Port of `ichiran/dict:get-hint` (`dict-split.lisp:938-945`).
///
/// Look up a hint function for `reading` in `*hint-map*`, applying the
/// first one that matches: first a direct lookup on the reading's own
/// seq, else walk every `from`-seq in the reading's conjugation data
/// and return the first hint that fires. `None` when neither path
/// produces a hint.
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

#[cfg(test)]
mod tests;
