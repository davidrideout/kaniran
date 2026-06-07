use super::helpers::{get_romaji_kana, romaji_kana_next};
use super::rules::apply_rmap_item;
use crate::characters::kana::as_katakana;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_info::find_kanji_for_pattern;
use serde_json::{Map, Value};
use std::collections::HashSet;

/// Port of `ichiran:rmap-item` (`deromanize.lisp:5`).
///
/// One romaji-to-kana rule from `data/romaji-map.csv`. `next` is non-empty
/// only for doubled-consonant gemination rows — e.g. `bb` emits `っ` and
/// re-emits `b` so the next pass picks up the second consonant.
#[derive(Debug, Clone)]
pub struct RmapItem {
    pub text: String,
    pub kana: String,
    pub next: Option<String>,
}

/// Port of `ichiran:has-successors` (`deromanize.lisp:13-19`).
///
/// Collects every proper prefix (char-wise, length 1 up to but not
/// including the full length) of each input string into a membership set.
pub fn has_successors(strings: &[&str]) -> HashSet<String> {
    let mut hash = HashSet::new();
    for s in strings {
        // (loop for end from 1 below (length s) for ss = (subseq s 0 end) ...)
        // subseq indexes by character, so prefixes are taken char-wise.
        let chars: Vec<char> = s.chars().collect();
        for end in 1..chars.len() {
            let ss: String = chars[..end].iter().collect();
            hash.insert(ss);
        }
    }
    hash
}

/// Port of `ichiran:kana-representation` (`deromanize.lisp:23`).
///
/// One branch of the deromanizer's candidate tree.
#[derive(Debug, Clone, Default)]
pub struct KanaRepresentation {
    pub canonical: String,
    pub pattern: String,
    pub rest: String,
    pub branch: i32,
}

/// Port of `ichiran:kr-concat` (`deromanize.lisp:25`).
///
/// Concatenates two deromanizer branches: appends `kr2`'s canonical
/// kana and pattern onto `kr1`'s, takes `kr2`'s remaining romaji as
/// the result `rest`, and keeps `kr1`'s `branch` tie-breaker.
pub fn kr_concat(kr1: &KanaRepresentation, kr2: &KanaRepresentation) -> KanaRepresentation {
    KanaRepresentation {
        canonical: format!("{}{}", kr1.canonical, kr2.canonical),
        pattern: format!("{}{}", kr1.pattern, kr2.pattern),
        rest: kr2.rest.clone(),
        branch: kr1.branch,
    }
}

/// Port of `ichiran:possible-long-vowel-p` (`deromanize.lisp:32`).
///
/// Returns the trailing `o` or `u` of `text`, or `None` when `text` is
/// empty or ends in any other character.
pub fn possible_long_vowel_p(text: &str) -> Option<char> {
    if text.is_empty() {
        return None;
    }
    let ch = text.chars().next_back().expect("text is non-empty here");
    ['o', 'u'].into_iter().find(|vowel| *vowel == ch)
}

/// Port of `ichiran:romaji-next` (`deromanize.lisp:46`).
///
/// For each prefix `ss` of `s` (growing one character at a time),
/// collects the applied romaji rule when one matches, stopping once `ss`
/// is no longer a proper prefix of any romaji key. The stop is checked
/// after the collect, so the prefix that ends the scan still contributes.
pub fn romaji_next(s: &str) -> Vec<KanaRepresentation> {
    let mut result = Vec::new();
    for end in 1..=s.chars().count() {
        let ss: String = s.chars().take(end).collect();
        if let Some(rmi) = get_romaji_kana(&ss) {
            result.push(apply_rmap_item(s, rmi));
        }
        if !romaji_kana_next().contains(ss.as_str()) {
            break;
        }
    }
    result
}

/// Port of `ichiran:join-branches` (`deromanize.lisp:55`).
///
/// Collapses sibling deromanizer branches that share the same remaining
/// input into one [`KanaRepresentation`] whose pattern is the alternation
/// `head(tail1|tail2|...)`. The canonical kana is the shortest branch
/// canonical (ties keep the first).
pub fn join_branches(branches: &[KanaRepresentation]) -> KanaRepresentation {
    let b0 = &branches[0];
    let tails: Vec<String> = branches
        .iter()
        .map(|b| b.pattern.chars().skip(b.branch as usize).collect())
        .collect();
    let head: String = b0.pattern.chars().take(b0.branch as usize).collect();
    let joined_kana = format!("{}({})", head, tails.join("|"));
    let canonical = branches
        .iter()
        .map(|b| &b.canonical)
        .reduce(|x, y| {
            if x.chars().count() <= y.chars().count() {
                x
            } else {
                y
            }
        })
        .expect("branches is non-empty")
        .clone();
    let branch = joined_kana.chars().count() as i32;
    KanaRepresentation {
        canonical,
        pattern: joined_kana,
        rest: b0.rest.clone(),
        branch,
    }
}

/// Port of `ichiran:branches-next` (`deromanize.lisp:69`).
///
/// Advances the deromanizer search one step: extends the first branch
/// with every `romaji-next` continuation of its remaining romaji,
/// appends the other branches, and sorts by remaining-romaji length
/// descending. A lone survivor has its `branch` index reset to its full
/// pattern length; when longest and shortest remaining romaji are equal,
/// the branches merge via `join-branches`.
pub fn branches_next(branches: &[KanaRepresentation]) -> Vec<KanaRepresentation> {
    let kr = &branches[0];
    // (nconc (loop for k in (romaji-next (kr-rest kr)) collect (kr-concat kr k)) (cdr branches))
    let mut new_branches: Vec<KanaRepresentation> = romaji_next(&kr.rest)
        .iter()
        .map(|k| kr_concat(kr, k))
        .collect();
    new_branches.extend(branches[1..].iter().cloned());
    // (sort … '> :key #'key) — key = (length (kr-rest b)); stable like SBCL's list merge sort
    new_branches.sort_by(|left, right| right.rest.chars().count().cmp(&left.rest.chars().count()));
    let new_len = new_branches.len();
    if new_len == 1 {
        // (setf (kr-branch (car new-branches)) (length (kr-pattern (car new-branches))))
        new_branches[0].branch = new_branches[0].pattern.chars().count() as i32;
    }
    if new_len > 1
        && new_branches[0].rest.chars().count() == new_branches[new_len - 1].rest.chars().count()
    {
        vec![join_branches(&new_branches)]
    } else {
        new_branches
    }
}

/// Port of `ichiran:romaji-kana` (`deromanize.lisp:84`).
///
/// Deromanizes `s`: steps the search until a branch consumes all input,
/// then returns its canonical kana paired with the anchored kana regex
/// `^pattern$` (`None` when the search exhausts without consuming).
pub fn romaji_kana(s: &str) -> Option<(String, String)> {
    let mut branches = vec![KanaRepresentation {
        rest: s.to_lowercase(),
        ..KanaRepresentation::default()
    }];
    let mut finished: Option<KanaRepresentation> = None;
    while !branches.is_empty() {
        branches = branches_next(&branches);
        if !branches.is_empty() && branches[0].rest.is_empty() {
            finished = Some(branches[0].clone());
            branches.clear();
        }
    }
    finished.map(|finished| (finished.canonical, format!("^{}$", finished.pattern)))
}

/// Port of `ichiran:romaji-suggest` (`deromanize.lisp:95`).
///
/// Deromanizes `s`, looks up the kanji and kana matching the resulting
/// kana pattern, and returns a `{"hiragana", "katakana", "kanji"}`
/// object (`None` when `s` does not deromanize).
pub async fn romaji_suggest(ctx: &KaniranContext, s: &str) -> Result<Option<Value>, sqlx::Error> {
    // (multiple-value-bind (canon pattern) (romaji-kana s) (when pattern …))
    let Some((canon, pattern)) = romaji_kana(s) else {
        return Ok(None);
    };
    // (multiple-value-bind (pkanji pkana) (find-kanji-for-pattern pattern) …)
    let (pkanji, pkana) = find_kanji_for_pattern(ctx, &pattern).await?;
    // (remove-duplicates (cons canon pkana) :test 'equal :from-end t)
    let mut hiragana_src = Vec::with_capacity(pkana.len() + 1);
    hiragana_src.push(canon);
    hiragana_src.extend(pkana);
    let hiragana = remove_duplicates_from_end(hiragana_src);
    // (jsown:new-js ("hiragana" …) ("katakana" (mapcar 'as-katakana …)) ("kanji" pkanji))
    let mut js = Map::new();
    js.insert(
        "hiragana".to_owned(),
        Value::Array(hiragana.iter().map(|h| Value::String(h.clone())).collect()),
    );
    js.insert(
        "katakana".to_owned(),
        Value::Array(
            hiragana
                .iter()
                .map(|h| Value::String(as_katakana(h)))
                .collect(),
        ),
    );
    js.insert(
        "kanji".to_owned(),
        Value::Array(pkanji.into_iter().map(Value::String).collect()),
    );
    Ok(Some(Value::Object(js)))
}

// `(remove-duplicates … :test 'equal :from-end t)` — keep the first
// occurrence of each value, preserving order.
fn remove_duplicates_from_end(items: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

#[cfg(test)]
mod tests;
