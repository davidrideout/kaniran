use crate::characters::char_class::{test_word, CharClass};
use crate::conn::kani_context::KaniranContext;
use crate::dict::load::conj_rules::{
    get_conj_rules, ConjugationRule, DO_NOT_CONJUGATE, DO_NOT_CONJUGATE_SEQ, POS_WITH_CONJ_RULES,
    SECONDARY_CONJUGATION_TYPES, SECONDARY_CONJUGATION_TYPES_FROM,
};
use crate::dict::load::pos::{get_pos, get_pos_index};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Port of `ichiran/dict:construct-conjugation` (`dict-load.lisp:284`).
///
/// Assemble a conjugated reading from a dictionary `word` and a
/// `ConjugationRule`: drop `stem` trailing characters (one extra when
/// the applicable euphonic fragment is non-empty), then append the
/// euphonic fragment (`euphr` when the last two characters are kana,
/// `euphk` otherwise) and `okuri`. Offsets are character-based.
pub fn construct_conjugation(word: &str, rule: &ConjugationRule) -> String {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    // (subseq word (max 0 (- (length word) 2))) — last two characters
    let last_two: String = chars[len.saturating_sub(2)..].iter().collect();
    let iskana = test_word(&last_two, CharClass::Kana);
    let euphr = &rule.euphr;
    let euphk = &rule.euphk;
    let stem = rule.stem
        + if (iskana && !euphr.is_empty()) || (!iskana && !euphk.is_empty()) {
            1
        } else {
            0
        };
    let mut result: String = chars[..len - stem as usize].iter().collect();
    result.push_str(if iskana { euphr } else { euphk });
    result.push_str(&rule.okuri);
    result
}

/// Port of `ichiran/dict:conjugate-word` (`dict-load.lisp:296`).
///
/// Returns the list of (rule, conjugated-form) pairs produced by
/// applying every conjugation rule registered for `pos` to `word`.
pub fn conjugate_word(word: &str, pos: &str) -> Vec<(ConjugationRule, String)> {
    let pos_id = match get_pos_index(pos) {
        Some(id) => id,
        // dict-load.lisp:297 — get-pos-index returns nil for unknown pos;
        // (get-conj-rules nil) returns nil; the loop collects nothing.
        None => return Vec::new(),
    };
    let rules = get_conj_rules(pos_id);
    // dict-load.lisp:299-300 (loop for rule in rules collect (cons rule (construct-conjugation word rule)))
    rules
        .into_iter()
        .map(|rule| {
            let conjugated = construct_conjugation(word, &rule);
            (rule, conjugated)
        })
        .collect()
}

/// Port of `ichiran/dict:conjugate-entry-inner` (`dict-load.lisp:316`).
///
/// Build the conjugation matrix for one entry. For every pos tag on the
/// entry (or from `as_posi` when supplied) the function looks up the
/// conj rules, fetches the conjugatable kanji/kana readings, applies
/// [`construct_conjugation`], and slots each result into a 2×2 array
/// indexed by `[neg][fml]` under the `(pos-id, conj-id)` key.
/// One row in a `ConjMatrix` cell — the 5-element list
/// `(conj-text kanji-flag reading ord onum)` pushed at
/// `dict-load.lisp:338-341`. Example value:
/// `("食べた".to_string(), 1, "食べる".to_string(), 0, 1)` — past-plain
/// form of 食べる from its `ord=0` kanji reading, rule `onum=1`.
pub type ConjMatrixEntry = (String, i32, String, i32, i32);

/// `(pos-id, conj-id) → 2×2 array` where index `[neg][fml]` holds the
/// list of [`ConjMatrixEntry`] rows produced for that combination.
/// Mirrors the upstream `(make-hash-table :test 'equal)` /
/// `(make-array '(2 2) :initial-element nil)` shape at
/// `dict-load.lisp:319/337`.
pub type ConjMatrix = HashMap<(i32, i32), [[Vec<ConjMatrixEntry>; 2]; 2]>;



/// Port of `ichiran/dict:lex-compare` (`dict-load.lisp:367`).
///
/// Returns a lexicographic comparator (a closure) parameterised on the
/// element-level `predicate`. Walks two equal-length sequences in
/// lockstep; the first pair where `predicate(e1, e2)` is true makes the
/// comparator return `true`, the first pair where `predicate(e2, e1)`
/// is true makes it return `false`. If neither holds for any pair, the
/// comparator returns `false`. Mismatched lengths walk only the shared
/// prefix and then return `false`.
pub fn lex_compare<T, P>(predicate: P) -> impl Fn(&[T], &[T]) -> bool
where
    P: Fn(&T, &T) -> bool,
{
    move |seq1, seq2| {
        // dict-load.lisp:371 (map nil (lambda (e1 e2) …) seq1 seq2)
        for (e1, e2) in seq1.iter().zip(seq2.iter()) {
            if predicate(e1, e2) {
                return true;
            }
            if predicate(e2, e1) {
                return false;
            }
        }
        // dict-load.lisp:370 (block nil …) — falls off `map nil` to nil.
        false
    }
}


/// Mirror CL's `(remove-duplicates seq :test 'equal)` default behaviour:
/// preserves the **last** occurrence of each value (the earlier
/// duplicates are dropped).
fn dedupe_keep_last<T: std::hash::Hash + Eq + Clone>(v: Vec<T>) -> Vec<T> {
    let mut seen: HashSet<T> = HashSet::new();
    let mut result: Vec<T> = Vec::with_capacity(v.len());
    for item in v.into_iter().rev() {
        if seen.insert(item.clone()) {
            result.push(item);
        }
    }
    result.reverse();
    result
}


/// Port of `ichiran/dict:load-secondary-conjugations` (`dict-load.lisp:460`).
///
/// Walks every primary conjugation tagged as a secondary type and drives
/// `conjugate_entry_outer` to build the second-order conjugations (`v5s`
/// posi for the causative-su source form, else `v1`); `from` restricts
/// the candidate set to the given source seqs.
// dict-errata.lisp:1239 (defconstant +conj-causative-su+ 53)
const CONJ_CAUSATIVE_SU: i32 = 53;


#[cfg(test)]
mod tests;
