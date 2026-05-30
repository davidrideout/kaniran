//! Romaji → kana deromanization. From `deromanize.lisp` (all
//! symbols).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use serde_json::{Map, Value};

use crate::characters::normalize::as_katakana;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_kanji_for_pattern::find_kanji_for_pattern;

// -- data shapes ----------------------------------------------------------

/// `rmap-item` (`deromanize.lisp:5`). In-memory record carrying one row
/// of `data/romaji-map.csv` — a romaji-to-kana rule. `text` is the
/// romaji prefix this rule consumes, `kana` the kana fragment it emits,
/// `next` the romaji fragment to prepend back onto the unconsumed input
/// after the rule fires (non-empty only for doubled-consonant
/// gemination rules — `bb` consumes `bb`, emits `っ`, re-emits `b`).
#[derive(Debug, Clone)]
pub struct RmapItem {
    pub text: String,
    pub kana: String,
    pub next: Option<String>,
}

/// `kana-representation` (`deromanize.lisp:23`). One branch of the
/// deromanizer's candidate tree: partial kana built so far
/// (`canonical`), the original romaji pattern that produced it
/// (`pattern`), the romaji still to consume (`rest`), and a per-branch
/// tie-breaker tag (`branch`).
#[derive(Debug, Clone, Default)]
pub struct KanaRepresentation {
    pub canonical: String,
    pub pattern: String,
    pub rest: String,
    pub branch: i32,
}

// -- tables: load + lookup ------------------------------------------------

const ROMAJI_MAP_CSV: &str = include_str!("../../data/romaji-map.csv");

/// `load-romaji-kana` (`deromanize.lisp:7`, `csv-hash *romaji-kana*`
/// expansion). Parse the vendored romaji-map.csv (tab-separated, no
/// header). Each row is `text<TAB>kana` with an optional third `next`
/// column on doubled-consonant rows. The `text` column is the key, so
/// the duplicate `fu` row collapses to one entry (292 keys from 293
/// rows).
pub fn load_romaji_kana() -> HashMap<String, RmapItem> {
    let mut hash = HashMap::new();
    for row in ROMAJI_MAP_CSV.lines() {
        let mut cols = row.split('\t');
        let text = cols.next().expect("romaji-map.csv row missing text column");
        let kana = cols.next().expect("romaji-map.csv row missing kana column");
        let next = cols.next();
        hash.insert(
            text.to_string(),
            RmapItem {
                text: text.to_string(),
                kana: kana.to_string(),
                next: next.map(str::to_string),
            },
        );
    }
    hash
}

/// `*romaji-kana*` (`deromanize.lisp:7`). Romaji prefix → [`RmapItem`]
/// rule. Lazily built once from the CSV.
pub fn romaji_kana_table() -> &'static HashMap<String, RmapItem> {
    static CACHE: OnceLock<HashMap<String, RmapItem>> = OnceLock::new();
    CACHE.get_or_init(load_romaji_kana)
}

/// `get-romaji-kana` (`deromanize.lisp:7`). Look up the romaji prefix
/// `key`. Upstream lazily fills `*romaji-kana*` on first call.
pub fn get_romaji_kana(key: &str) -> Option<&'static RmapItem> {
    romaji_kana_table().get(key)
}

/// `has-successors` (`deromanize.lisp:13-19`). Every proper prefix
/// (length 1 up to but not including the full length) of each input.
/// Upstream returns a `:test 'equal` hash whose values are all `t`,
/// consulted only as a membership test.
pub fn has_successors(strings: &[&str]) -> HashSet<String> {
    let mut hash = HashSet::new();
    for s in strings {
        let chars: Vec<char> = s.chars().collect();
        for end in 1..chars.len() {
            let ss: String = chars[..end].iter().collect();
            hash.insert(ss);
        }
    }
    hash
}

/// `*romaji-kana-next*` (`deromanize.lisp:21`). Set of every proper
/// prefix of every romaji key in `*romaji-kana*` — the
/// "could-this-grow-into-a-longer-key" membership test consulted by
/// `romaji-next`.
pub fn romaji_kana_next() -> &'static HashSet<String> {
    static CACHE: OnceLock<HashSet<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let keys: Vec<&str> = romaji_kana_table().keys().map(String::as_str).collect();
        has_successors(&keys)
    })
}

// -- helpers --------------------------------------------------------------

/// `possible-long-vowel-p` (`deromanize.lisp:32`). The trailing `o` or
/// `u` of `text`, or `None` for empty or any other ending.
pub fn possible_long_vowel_p(text: &str) -> Option<char> {
    if text.is_empty() {
        return None;
    }
    let ch = text.chars().next_back().expect("text is non-empty here");
    ['o', 'u'].into_iter().find(|vowel| *vowel == ch)
}

/// `apply-rmap-item` (`deromanize.lisp:37`). Build the
/// [`KanaRepresentation`] for applying one rule `rmi` to input `s`:
/// the rule's kana becomes the canonical kana and the base pattern,
/// with a trailing `う?` when the consumed romaji could be a long
/// vowel; `rest` is `rmi.next` (or empty) prepended to the post-prefix
/// tail of `s`.
pub fn apply_rmap_item(s: &str, rmi: &RmapItem) -> KanaRepresentation {
    let kana = &rmi.kana;
    KanaRepresentation {
        canonical: kana.clone(),
        pattern: if possible_long_vowel_p(&rmi.text).is_some() {
            format!("{kana}う?")
        } else {
            kana.clone()
        },
        rest: format!(
            "{}{}",
            rmi.next.as_deref().unwrap_or(""),
            s.chars().skip(rmi.text.chars().count()).collect::<String>()
        ),
        branch: 0,
    }
}

/// `kr-concat` (`deromanize.lisp:25`). Append `kr2`'s canonical/pattern
/// onto `kr1`'s; take `kr2`'s `rest`, keep `kr1`'s `branch`.
pub fn kr_concat(kr1: &KanaRepresentation, kr2: &KanaRepresentation) -> KanaRepresentation {
    KanaRepresentation {
        canonical: format!("{}{}", kr1.canonical, kr2.canonical),
        pattern: format!("{}{}", kr1.pattern, kr2.pattern),
        rest: kr2.rest.clone(),
        branch: kr1.branch,
    }
}

/// `romaji-next` (`deromanize.lisp:46`). For each growing prefix `ss`
/// of `s`, collect the applied rule when one matches, stopping once
/// `ss` is no longer a proper prefix of any romaji key. The `while`
/// runs after the collect, so the prefix that ends the scan still
/// contributes.
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

/// `join-branches` (`deromanize.lisp:55`). Collapse sibling branches
/// sharing the same remaining input into one [`KanaRepresentation`]
/// whose pattern is `head(tail1|tail2|...)` where `head` is the common
/// prefix (up to the first branch's `branch` index) and each `tail` is
/// a branch pattern past that index. Canonical is the shortest branch
/// canonical (`<=`, ties keep the first); `rest` from the first;
/// `branch` becomes the joined pattern's char length.
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
        .reduce(|x, y| if x.chars().count() <= y.chars().count() { x } else { y })
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

/// `branches-next` (`deromanize.lisp:69`). Advance the search one step:
/// extend the first branch with every `romaji-next` continuation of
/// its remaining romaji, append the other branches, sort by remaining
/// length descending. A lone surviving branch has its `branch` index
/// reset to its full pattern length; when longest and shortest
/// remaining are equal, the branches merge via [`join_branches`].
pub fn branches_next(branches: &[KanaRepresentation]) -> Vec<KanaRepresentation> {
    let kr = &branches[0];
    let mut new_branches: Vec<KanaRepresentation> =
        romaji_next(&kr.rest).iter().map(|k| kr_concat(kr, k)).collect();
    new_branches.extend(branches[1..].iter().cloned());
    new_branches.sort_by(|left, right| right.rest.chars().count().cmp(&left.rest.chars().count()));
    let new_len = new_branches.len();
    if new_len == 1 {
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

/// `romaji-kana` (`deromanize.lisp:84`). Deromanize `s`: seed one
/// branch holding the lowercased input, step the search via
/// [`branches_next`] until a branch consumes all input, return its
/// canonical kana paired with the anchored kana regex `^pattern$`.
/// `None` when the search exhausts without consuming the input.
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

/// `romaji-suggest` (`deromanize.lisp:95`). Deromanize `s`, look up
/// kanji + kana matching the kana pattern, return a
/// `{"hiragana", "katakana", "kanji"}` object. `hiragana` is the
/// canonical kana followed by the pattern's kana readings with
/// duplicates removed (first kept); `katakana` is each of those as
/// katakana; `kanji` is the matched kanji. `None` when `s` does not
/// deromanize.
pub async fn romaji_suggest(ctx: &KaniranContext, s: &str) -> Result<Option<Value>, sqlx::Error> {
    let Some((canon, pattern)) = romaji_kana(s) else {
        return Ok(None);
    };
    let (pkanji, pkana) = find_kanji_for_pattern(ctx, &pattern).await?;
    let mut hiragana_src = Vec::with_capacity(pkana.len() + 1);
    hiragana_src.push(canon);
    hiragana_src.extend(pkana);
    let hiragana = remove_duplicates_from_end(hiragana_src);
    let mut js = Map::new();
    js.insert(
        "hiragana".to_owned(),
        Value::Array(hiragana.iter().map(|h| Value::String(h.clone())).collect()),
    );
    js.insert(
        "katakana".to_owned(),
        Value::Array(hiragana.iter().map(|h| Value::String(as_katakana(h))).collect()),
    );
    js.insert(
        "kanji".to_owned(),
        Value::Array(pkanji.into_iter().map(Value::String).collect()),
    );
    Ok(Some(Value::Object(js)))
}

/// `(remove-duplicates … :test 'equal :from-end t)` — keep the first
/// occurrence of each value, preserving order.
fn remove_duplicates_from_end(items: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    items.into_iter().filter(|item| seen.insert(item.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103, after `(load-romaji-kana)`): 292 keys (293 rows, `fu`
    /// duplicated).
    #[test]
    fn loads_romaji_map_csv() {
        let map = load_romaji_kana();
        assert_eq!(map.len(), 292);

        let a = &map["a"];
        assert_eq!((a.text.as_str(), a.kana.as_str(), a.next.as_deref()), ("a", "あ", None));
        let n = &map["n"];
        assert_eq!((n.text.as_str(), n.kana.as_str(), n.next.as_deref()), ("n", "ん", None));
        let di = &map["d'i"];
        assert_eq!((di.text.as_str(), di.kana.as_str(), di.next.as_deref()), ("d'i", "でぃ", None));
        let bb = &map["bb"];
        assert_eq!((bb.text.as_str(), bb.kana.as_str(), bb.next.as_deref()), ("bb", "っ", Some("b")));
        let mm = &map["mm"];
        assert_eq!((mm.text.as_str(), mm.kana.as_str(), mm.next.as_deref()), ("mm", "ん", Some("m")));
    }

    #[test]
    fn romaji_kana_table_builds_once() {
        assert_eq!(romaji_kana_table().len(), 292);
    }

    /// REPL (.103): `(hash-table-count *romaji-kana-next*)` = 60.
    #[test]
    fn romaji_kana_next_builds_once() {
        let next = romaji_kana_next();
        assert_eq!(next.len(), 60);
        for present in ["k", "ky", "c", "ch", "b", "s", "sh"] {
            assert!(next.contains(present), "expected {present:?} present");
        }
        for absent in ["ka", "a", "zz"] {
            assert!(!next.contains(absent), "expected {absent:?} absent");
        }
    }

    /// REPL fixtures (.103, ichiran::get-romaji-kana), 2026-05-26.
    #[test]
    fn get_romaji_kana_fixtures() {
        let cases: &[(&str, Option<(&str, &str, Option<&str>)>)] = &[
            ("a", Some(("a", "あ", None))),
            ("ka", Some(("ka", "か", None))),
            ("shi", Some(("shi", "し", None))),
            ("n", Some(("n", "ん", None))),
            ("bb", Some(("bb", "っ", Some("b")))),
            ("kk", Some(("kk", "っ", Some("k")))),
            ("pp", Some(("pp", "っ", Some("p")))),
            ("xyz", None),
            ("", None),
        ];
        for (key, expected) in cases {
            let got = get_romaji_kana(key)
                .map(|rmi| (rmi.text.as_str(), rmi.kana.as_str(), rmi.next.as_deref()));
            assert_eq!(got, *expected, "key={key:?}");
        }
    }

    /// REPL fixtures (.103, ichiran::has-successors), 2026-05-25.
    #[test]
    fn has_successors_fixtures() {
        fn set(items: &[&str]) -> HashSet<String> {
            items.iter().map(|item| item.to_string()).collect()
        }
        let cases: &[(&[&str], &[&str])] = &[
            (&["cha", "chi", "ba"], &["c", "ch", "b"]),
            (&["a", "x", ""], &[]),
            (&["kya", "kyo", "sha"], &["k", "ky", "s", "sh"]),
        ];
        for (input, expected) in cases {
            assert_eq!(has_successors(input), set(expected), "input={input:?}");
        }
    }

    /// REPL fixtures (.103, ichiran::possible-long-vowel-p), 2026-05-25.
    #[test]
    fn possible_long_vowel_p_fixtures() {
        let cases: &[(&str, Option<char>)] = &[
            ("", None),
            ("ko", Some('o')),
            ("ku", Some('u')),
            ("ka", None),
            ("o", Some('o')),
            ("u", Some('u')),
            ("shinbun", None),
            ("kyou", Some('u')),
            ("toukyou", Some('u')),
            ("sapporo", Some('o')),
            ("gakkou", Some('u')),
            ("arigatou", Some('u')),
            ("tomodachi", None),
            ("fujisan", None),
            ("おう", None),
            ("あo", Some('o')),
            ("katsu", Some('u')),
        ];
        for (text, expected) in cases {
            assert_eq!(possible_long_vowel_p(text), *expected, "text={text:?}");
        }
    }

    fn rmi(text: &str, kana: &str, next: Option<&str>) -> RmapItem {
        RmapItem {
            text: text.to_string(),
            kana: kana.to_string(),
            next: next.map(str::to_string),
        }
    }

    /// REPL fixtures (.103, ichiran::apply-rmap-item), 2026-05-26.
    #[test]
    fn apply_rmap_item_fixtures() {
        let cases: &[(&str, RmapItem, (&str, &str, &str))] = &[
            ("konnichiwa", rmi("ko", "こ", None), ("こ", "こう?", "nnichiwa")),
            ("ohayou", rmi("o", "お", None), ("お", "おう?", "hayou")),
            ("toukyou", rmi("to", "と", None), ("と", "とう?", "ukyou")),
            ("katana", rmi("ka", "か", None), ("か", "か", "tana")),
            ("shinkansen", rmi("shi", "し", None), ("し", "し", "nkansen")),
            ("nagoya", rmi("na", "な", None), ("な", "な", "goya")),
            ("kkon", rmi("kk", "っ", Some("k")), ("っ", "っ", "kon")),
            ("ppai", rmi("pp", "っ", Some("p")), ("っ", "っ", "pai")),
            ("kkou", rmi("kk", "っ", Some("k")), ("っ", "っ", "kou")),
        ];
        for (s, item, (canonical, pattern, rest)) in cases {
            let got = apply_rmap_item(s, item);
            assert_eq!(got.canonical, *canonical, "canonical, s={s:?} rmi={item:?}");
            assert_eq!(got.pattern, *pattern, "pattern, s={s:?} rmi={item:?}");
            assert_eq!(got.rest, *rest, "rest, s={s:?} rmi={item:?}");
            assert_eq!(got.branch, 0, "branch, s={s:?} rmi={item:?}");
        }
    }

    fn kr(canonical: &str, pattern: &str, rest: &str, branch: i32) -> KanaRepresentation {
        KanaRepresentation {
            canonical: canonical.to_string(),
            pattern: pattern.to_string(),
            rest: rest.to_string(),
            branch,
        }
    }

    /// REPL fixtures (.103, ichiran::kr-concat), 2026-05-26.
    #[test]
    fn kr_concat_fixtures() {
        let cases: &[(KanaRepresentation, KanaRepresentation, KanaRepresentation)] = &[
            (
                kr("な", "な", "goya", 0),
                kr("ご", "ごう?", "ya", 0),
                kr("なご", "なごう?", "ya", 0),
            ),
            (
                kr("こ", "こう?", "nnichiwa", 0),
                kr("ん", "ん", "nichiwa", 0),
                kr("こん", "こう?ん", "nichiwa", 0),
            ),
            (
                kr("さ", "さあ?", "old", 2),
                kr("ん", "ん", "new", 9),
                kr("さん", "さあ?ん", "new", 2),
            ),
            (
                kr("あ", "あ", "iueo", 0),
                kr("い", "い", "ueo", 7),
                kr("あい", "あい", "ueo", 0),
            ),
        ];
        for (kr1, kr2, expected) in cases {
            let got = kr_concat(kr1, kr2);
            assert_eq!(got.canonical, expected.canonical, "canonical, kr1={kr1:?} kr2={kr2:?}");
            assert_eq!(got.pattern, expected.pattern, "pattern, kr1={kr1:?} kr2={kr2:?}");
            assert_eq!(got.rest, expected.rest, "rest, kr1={kr1:?} kr2={kr2:?}");
            assert_eq!(got.branch, expected.branch, "branch, kr1={kr1:?} kr2={kr2:?}");
        }
    }

    /// REPL fixtures (.103, ichiran::romaji-next), 2026-05-26.
    #[test]
    fn romaji_next_fixtures() {
        fn show(krs: &[KanaRepresentation]) -> Vec<(String, String, String, i32)> {
            krs.iter()
                .map(|kr| (kr.canonical.clone(), kr.pattern.clone(), kr.rest.clone(), kr.branch))
                .collect()
        }
        let cases: &[(&str, Vec<(&str, &str, &str, i32)>)] = &[
            ("tokyo", vec![("と", "とう?", "kyo", 0)]),
            ("chotto", vec![("ちょ", "ちょう?", "tto", 0)]),
            ("kkou", vec![("っ", "っ", "kou", 0)]),
            ("shinbun", vec![("し", "し", "nbun", 0)]),
            ("arigatou", vec![("あ", "あ", "rigatou", 0)]),
            ("nippon", vec![("ん", "ん", "ippon", 0), ("に", "に", "ppon", 0)]),
            ("xyz", vec![]),
            ("", vec![]),
        ];
        for (s, expected) in cases {
            let exp: Vec<(String, String, String, i32)> = expected
                .iter()
                .map(|(canonical, pattern, rest, branch)| {
                    (canonical.to_string(), pattern.to_string(), rest.to_string(), *branch)
                })
                .collect();
            assert_eq!(show(&romaji_next(s)), exp, "s={s:?}");
        }
    }

    /// REPL fixtures (.103, ichiran::join-branches), 2026-05-25.
    #[test]
    fn join_branches_fixtures() {
        let cases: &[(Vec<KanaRepresentation>, KanaRepresentation)] = &[
            (
                vec![kr("んあ", "んあ", "goya", 0), kr("な", "な", "goya", 0)],
                kr("な", "(んあ|な)", "goya", 6),
            ),
            (
                vec![
                    kr("こんんい", "こう?んんい", "chiwa", 4),
                    kr("こんに", "こう?んに", "chiwa", 4),
                ],
                kr("こんに", "こう?ん(んい|に)", "chiwa", 10),
            ),
            (
                vec![kr("あ", "あ?", "x", 1), kr("いう", "いう", "x", 1)],
                kr("あ", "あ(?|う)", "x", 6),
            ),
            (
                vec![kr("かき", "かき", "yo", 0), kr("くけ", "くけ", "yo", 0)],
                kr("かき", "(かき|くけ)", "yo", 7),
            ),
            (
                vec![
                    kr("さん", "さあ?ん", "z", 2),
                    kr("さに", "さあ?に", "z", 2),
                    kr("さ", "さあ?ぬ", "z", 2),
                ],
                kr("さ", "さあ(?ん|?に|?ぬ)", "z", 12),
            ),
        ];
        for (branches, expected) in cases {
            let got = join_branches(branches);
            assert_eq!(got.canonical, expected.canonical, "canonical, branches={branches:?}");
            assert_eq!(got.pattern, expected.pattern, "pattern, branches={branches:?}");
            assert_eq!(got.rest, expected.rest, "rest, branches={branches:?}");
            assert_eq!(got.branch, expected.branch, "branch, branches={branches:?}");
        }
    }

    /// REPL fixtures (.103, ichiran::branches-next), 2026-05-26.
    #[test]
    fn branches_next_fixtures() {
        fn show(krs: &[KanaRepresentation]) -> Vec<(String, String, String, i32)> {
            krs.iter()
                .map(|item| (item.canonical.clone(), item.pattern.clone(), item.rest.clone(), item.branch))
                .collect()
        }
        let cases: &[(Vec<KanaRepresentation>, Vec<KanaRepresentation>)] = &[
            (
                vec![kr("", "", "nippon", 0)],
                vec![kr("ん", "ん", "ippon", 0), kr("に", "に", "ppon", 0)],
            ),
            (
                vec![kr("ん", "ん", "ippon", 0), kr("に", "に", "ppon", 0)],
                vec![kr("に", "(んい|に)", "ppon", 6)],
            ),
            (
                vec![kr("に", "(んい|に)", "ppon", 6)],
                vec![kr("にっ", "(んい|に)っ", "pon", 7)],
            ),
            (
                vec![kr("にっ", "(んい|に)っ", "pon", 7)],
                vec![kr("にっぽ", "(んい|に)っぽう?", "n", 10)],
            ),
            (
                vec![kr("にっぽ", "(んい|に)っぽう?", "n", 10)],
                vec![kr("にっぽん", "(んい|に)っぽう?ん", "", 11)],
            ),
            (
                vec![kr("", "", "konnichiwa", 0)],
                vec![kr("こ", "こう?", "nnichiwa", 3)],
            ),
            (
                vec![kr("こん", "こう?ん", "nichiwa", 4)],
                vec![
                    kr("こんん", "こう?んん", "ichiwa", 4),
                    kr("こんに", "こう?んに", "chiwa", 4),
                ],
            ),
            (
                vec![
                    kr("こんん", "こう?んん", "ichiwa", 4),
                    kr("こんに", "こう?んに", "chiwa", 4),
                ],
                vec![kr("こんに", "こう?ん(んい|に)", "chiwa", 10)],
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(show(&branches_next(input)), show(expected), "in={input:?}");
        }
    }

    /// REPL fixtures (.103, ichiran::romaji-kana), 2026-05-26.
    #[test]
    fn romaji_kana_fixtures() {
        let cases: &[(&str, Option<(&str, &str)>)] = &[
            ("nippon", Some(("にっぽん", "^(んい|に)っぽう?ん$"))),
            ("konnichiwa", Some(("こんにちわ", "^こう?ん(んい|に)ちわ$"))),
            ("gakkou", Some(("がっこう", "^がっこう?うう?$"))),
            ("tokyo", Some(("ときょ", "^とう?きょう?$"))),
            ("chuui", Some(("ちゅうい", "^ちゅう?うう?い$"))),
            ("sakura", Some(("さくら", "^さくう?ら$"))),
            ("n", Some(("ん", "^ん$"))),
            ("a", Some(("あ", "^あ$"))),
            ("TOKYO", Some(("ときょ", "^とう?きょう?$"))),
            ("Nippon", Some(("にっぽん", "^(んい|に)っぽう?ん$"))),
            ("xyz", None),
            ("tt", None),
            ("qz", None),
            ("", None),
        ];
        for (input, expected) in cases {
            let got = romaji_kana(input);
            let exp = expected.map(|(canonical, pattern)| (canonical.to_string(), pattern.to_string()));
            assert_eq!(got, exp, "input={input:?}");
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `jsown:to-json` of `(romaji-suggest s)`),
    /// 2026-05-26.
    #[tokio::test]
    async fn romaji_suggest_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, &str)] = &[
            ("neko", r#"{"hiragana":["ねこ"],"katakana":["ネコ"],"kanji":["猫"]}"#),
            ("tegami", r#"{"hiragana":["てがみ"],"katakana":["テガミ"],"kanji":["手紙"]}"#),
            ("toukyou", r#"{"hiragana":["とうきょう"],"katakana":["トウキョウ"],"kanji":["東京"]}"#),
            ("hon", r#"{"hiragana":["ほん"],"katakana":["ホン"],"kanji":["本","品"]}"#),
        ];
        for (s, expected) in cases {
            let js = romaji_suggest(&ctx, s).await.unwrap().expect("deromanizes");
            assert_eq!(serde_json::to_string(&js).unwrap().as_str(), *expected, "s={s}");
        }
    }

    /// Matched-kanji order is DB-scan-dependent for tied `common`; pin
    /// only the deterministic hiragana/katakana fields.
    #[tokio::test]
    async fn romaji_suggest_multi_hiragana() {
        let ctx = ctx().await;
        let js = romaji_suggest(&ctx, "inu").await.unwrap().expect("deromanizes");
        assert_eq!(js["hiragana"], serde_json::json!(["いぬ", "いんう"]));
        assert_eq!(js["katakana"], serde_json::json!(["イヌ", "インウ"]));
    }

    #[tokio::test]
    async fn romaji_suggest_no_parse() {
        let ctx = ctx().await;
        assert!(romaji_suggest(&ctx, "xyz").await.unwrap().is_none());
    }
}
