use crate::conn::kani_backend::KaniBackend;
use crate::conn::kani_context::KaniranContext;
use crate::dict::counters::methods::seq;
use crate::dict::grammar::suffix::constants::{
    lookup_suffix_fn, suffix_class, SuffixUniqueOnly, SUFFIX_UNIQUE_ONLY,
};
use crate::dict::dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::scoring::score::subseq_slice;
use crate::dict::word_info::WordInfoSeq;
use std::collections::HashMap;

/// Port of `ichiran/dict:parse-suffix-val` (`dict-grammar.lisp:665`).
///
/// Pairs `substr` with each cache entry under that substring, yielding
/// a flat list of `(substr, key, kf)` triples.
pub fn parse_suffix_val<'a, 'b>(
    substr: &'a str,
    val: Option<&'b [(String, Option<KanaText>)]>,
) -> Vec<(&'a str, &'b str, Option<&'b KanaText>)> {
    match val {
        None => Vec::new(),
        Some(entries) => entries
            .iter()
            .map(|(key, kf)| (substr, key.as_str(), kf.as_ref()))
            .collect(),
    }
}

/// Port of `ichiran/dict:get-suffix-map` (`dict-grammar.lisp:671`).
///
/// For every substring of `str`, looks up the suffix cache and maps
/// each parsed suffix item under the substring's end position. Loop
/// bounds and `subseq-slice` are character offsets, not byte offsets.
pub fn get_suffix_map<'a, 'b>(
    ctx: &'a KaniranContext,
    str: &'b str,
) -> HashMap<usize, Vec<(&'b str, &'a str, Option<&'a KanaText>)>> {
    // dict-grammar.lisp:672 (init-suffixes) — eager-population invariant
    // of KaniranContext::from_url.
    assert!(
        !ctx.suffix_cache.is_empty(),
        "get-suffix-map: ctx.suffix_cache is empty (init-suffixes contract violated)",
    );
    let mut result: HashMap<usize, Vec<(&'b str, &'a str, Option<&'a KanaText>)>> = HashMap::new();
    let length = str.chars().count();
    for start in 0..length {
        for end in (start + 1)..=length {
            let substr = subseq_slice(None, str, start, Some(end));
            let val = ctx.suffix_cache.get(substr).map(|v| v.as_slice());
            for item in parse_suffix_val(substr, val) {
                // dict-grammar.lisp:679 (push item (gethash end result nil))
                result.entry(end).or_default().insert(0, item);
            }
        }
    }
    result
}

/// Port of `ichiran/dict:get-suffixes` (`dict-grammar.lisp:682`).
///
/// Enumerates every grammatical-suffix match anchored at any non-zero
/// offset of `word`, ordered so that matches at higher `start`
/// (shorter suffixes) come first.
pub fn get_suffixes<'a, 'b>(
    ctx: &'a KaniranContext,
    word: &'b str,
) -> Vec<(&'b str, &'a str, Option<&'a KanaText>)> {
    // dict-grammar.lisp:683 (init-suffixes) — eager-population invariant
    // of `KaniranContext::from_url`. The contract is load-bearing: an
    // unpopulated cache makes get_suffixes silently degenerate to the
    // empty answer for every word, which would corrupt every segmenter
    // path that depends on suffix recognition. Guard in release too.
    assert!(
        !ctx.suffix_cache.is_empty(),
        "get-suffixes: ctx.suffix_cache is empty (init-suffixes contract violated)",
    );
    let len = word.chars().count();
    let mut out: Vec<(&'b str, &'a str, Option<&'a KanaText>)> = Vec::new();
    // dict-grammar.lisp:684 (loop for start from (1- (length word)) downto 1)
    for start in (1..len).rev() {
        // dict-grammar.lisp:685 (subseq-slice nil word start)
        let substr = subseq_slice(None, word, start, None);
        // dict-grammar.lisp:686 (gethash substr *suffix-cache*)
        let val = ctx.suffix_cache.get(substr).map(|v| v.as_slice());
        // dict-grammar.lisp:687 (nconc (parse-suffix-val substr val))
        out.extend(parse_suffix_val(substr, val));
    }
    out
}

/// Port of `ichiran/dict:match-unique` (`dict-grammar.lisp:689`).
///
/// Consults [`SUFFIX_UNIQUE_ONLY`] for `suffix_class`:
///
/// - Not in the table → `None`.
/// - Bare entry → `Some(MatchUniqueResult::Bare)`.
/// - `:desu` entry → a `conjugation` query for matches derived from seq
///   2755350 (じゃない); `Desu` when at least one match is not a じゃない
///   conjugation, else `None`.
/// - `:sa` entry → an `entry` query for the matches' seqs filtered by
///   `root_p`; `Sa(rows)` when non-empty, else `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchUniqueResult {
    Bare,
    Sa(Vec<i32>),
    Desu,
}

pub fn match_unique(
    ctx: &KaniranContext,
    suffix_class: &str,
    matches: &[KaniWordDispatchEnum],
) -> Result<Option<MatchUniqueResult>, crate::conn::KaniDbError> {
    let Some((_, kind)) = SUFFIX_UNIQUE_ONLY.iter().find(|(k, _)| *k == suffix_class) else {
        return Ok(None);
    };
    match kind {
        // dict-grammar.lisp:689-693 (defun match-unique) — bare entry
        // case: `(cond ... (t uniq))` returns the matched keyword
        // itself (truthy).
        SuffixUniqueOnly::Bare => Ok(Some(MatchUniqueResult::Bare)),
        // dict-grammar.lisp:522-530 (pushnew (cons :desu (lambda …)))
        SuffixUniqueOnly::Desu => {
            let seqs = collect_seqs(matches);
            // (and seqs (query …)) — empty seqs short-circuit to nil
            // → (length nil) = 0 → (< 0 (length matches)) = T iff
            // matches non-empty.
            let row_count = if seqs.is_empty() {
                0
            } else {
                let rows: Vec<i32> = ctx.store.conj_seqs_of_desu(&seqs)?;
                rows.len()
            };
            Ok(if row_count < matches.len() {
                Some(MatchUniqueResult::Desu)
            } else {
                None
            })
        }
        // dict-grammar.lisp:486-490 (pushnew (cons :sa (lambda …)))
        SuffixUniqueOnly::Sa => {
            let seqs = collect_seqs(matches);
            if seqs.is_empty() {
                // (and nil …) → nil → None.
                return Ok(None);
            }
            let rows: Vec<i32> = ctx.store.root_seqs(&seqs)?;
            // (and seqs (query …)) — non-empty seqs evaluate the
            // query; empty result is nil (falsy), non-empty is the
            // list of root seqs (truthy).
            Ok(if rows.is_empty() {
                None
            } else {
                Some(MatchUniqueResult::Sa(rows))
            })
        }
    }
}

/// dict-grammar.lisp:488,524 (loop for match in matches if (seq match)
/// collect it) — collects the integer seq when `(seq match)` is
/// truthy, skips when nil. `Multi` panics per the module-doc edge-
/// case note (mirrors Lisp's Postgres 42883 error).
fn collect_seqs(matches: &[KaniWordDispatchEnum]) -> Vec<i32> {
    matches
        .iter()
        .filter_map(|m| match seq(m) {
            Some(WordInfoSeq::Single(s)) => Some(s),
            None => None,
            Some(WordInfoSeq::Multi(_)) => panic!(
                "match_unique: compound-text seq returned WordInfoSeq::Multi; \
                 upstream Lisp errors with Postgres 42883 \
                 'operator does not exist: integer = record' for this input"
            ),
        })
        .collect()
}

/// Port of `ichiran/dict:find-word-suffix` (`dict-grammar.lisp:695`).
///
/// Iterates over the suffix triples for `word` (from the precomputed
/// suffix-map-temp when bound, else [`get_suffixes`]), dispatches each
/// through [`SUFFIX_LIST`], and concatenates the resulting rows. Each
/// suffix-fn runs with `suffix_next_end` decremented by the suffix's
/// character length so a nested call can peel further. Offsets are
/// character positions, not bytes.
///
/// For the match-unique gate, the suffix-class is the value in
/// [`SUFFIX_CLASS`] for `kf.seq` when `kf` is present, otherwise the
/// keyword itself.
///
/// [`SUFFIX_LIST`]: crate::dict::grammar::suffix::constants::SUFFIX_LIST
/// [`get_suffixes`]: crate::dict::grammar::suffix::resolve::get_suffixes
/// [`SUFFIX_CLASS`]: crate::dict::grammar::suffix::constants::suffix_class
pub fn find_word_suffix(
    ctx: &KaniranContext,
    word: &str,
    matches: &[KaniWordDispatchEnum],
) -> Result<Vec<KaniWordDispatchEnum>, crate::conn::KaniDbError> {
    let word_len = word.chars().count();

    // dict-grammar.lisp:696-698 (with suffixes = (if *suffix-map-temp* …))
    // Two sources keep distinct ownership: a borrowed slice over the
    // ctx-owned map, or an owned Vec from get_suffixes.
    let suffixes_owned: Vec<(String, String, Option<KanaText>)>;
    let suffixes_from_map: Option<&[(String, String, Option<KanaText>)]>;

    if let Some(map) = ctx.suffix_map_temp.as_deref() {
        // dict-grammar.lisp:697 (gethash *suffix-next-end* *suffix-map-temp*)
        // Negative *suffix-next-end* values match no hash key (gethash
        // returns nil); SuffixMapTemp keys are usize so we mirror via
        // `try_from`.
        let key = ctx.suffix_next_end.and_then(|e| usize::try_from(e).ok());
        suffixes_from_map = key.and_then(|k| map.get(&k)).map(|v| v.as_slice());
        suffixes_owned = Vec::new();
    } else {
        // dict-grammar.lisp:698 (get-suffixes word)
        // get_suffixes returns borrowed slices into ctx; convert to
        // owned triples so the loop body can be uniform.
        suffixes_owned = get_suffixes(ctx, word)
            .into_iter()
            .map(|(s, k, kf)| (s.to_string(), k.to_string(), kf.cloned()))
            .collect();
        suffixes_from_map = None;
    }
    let suffix_triples: &[(String, String, Option<KanaText>)] = match suffixes_from_map {
        Some(s) => s,
        None => suffixes_owned.as_slice(),
    };

    let class_map = suffix_class(ctx);
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();

    // dict-grammar.lisp:700 (for (suffix keyword kf) in suffixes)
    for (suffix, keyword, kf) in suffix_triples {
        // dict-grammar.lisp:701 (cdr (assoc keyword *suffix-list*))
        let Some(suffix_fn) = lookup_suffix_fn(keyword) else {
            continue;
        };
        let suffix_len = suffix.chars().count();
        // dict-grammar.lisp:703 (- (length word) (length suffix))
        // Use checked_sub to mirror Lisp's signed arithmetic — Lisp
        // would produce a negative offset for over-long suffixes,
        // failing the (> offset 0) gate; in Rust we treat the
        // saturating-to-zero case the same way.
        let Some(offset) = word_len.checked_sub(suffix_len) else {
            continue;
        };
        // dict-grammar.lisp:704 (and suffix-fn (> offset 0) ...)
        if offset == 0 {
            continue;
        }
        // dict-grammar.lisp:702 (if kf (gethash (seq kf) *suffix-class*) keyword)
        let suffix_class_str: &str = match kf {
            Some(k) => class_map.get(&k.seq).map(String::as_str).unwrap_or(keyword),
            None => keyword,
        };
        // dict-grammar.lisp:705 (not (and matches (match-unique suffix-class matches)))
        if !matches.is_empty()
            && match_unique(ctx, suffix_class_str, matches)
                ?
                .is_some()
        {
            continue;
        }
        // dict-grammar.lisp:706 (let ((*suffix-next-end* (and *suffix-next-end* (- *suffix-next-end* (length suffix)))))
        // Lisp `(and nil x)` → nil so the rebind only computes a new
        // value when the current binding is non-nil. Mirror with
        // `Option::map`.
        let new_next_end = ctx.suffix_next_end.map(|e| e - suffix_len as i32);
        let ctx2 = ctx.with_suffix_next_end(new_next_end);
        // dict-grammar.lisp:707 (funcall suffix-fn (subseq-slice slice word 0 offset) suffix kf)
        // subseq_slice's first arg is the legacy `make-slice` seed and
        // is ignored in Rust; pass None per the port note.
        let root = subseq_slice(None, word, 0, Some(offset));
        let compounds = suffix_fn(&ctx2, root, suffix, kf.as_ref())?;
        out.extend(compounds);
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
