use super::conj::ConjData;
use super::dao::ConjProp;
use super::load::conj_rules::ConjugationRule;
use super::load::pos::get_pos;
use super::load::pos::get_pos_index;
use super::kani_conj_form::{ConjForm, FormToken};
use std::collections::HashMap;
use std::sync::OnceLock;








/// Port of `ichiran/dict:root-diff` (`dict-errata.lisp:95`).
///
/// Counts how many leading characters of `base_text` and `reading`
/// sit outside their shared right-aligned tail.
pub fn root_diff(base_text: &str, reading: &str) -> (usize, usize) {
    let base_chars: Vec<char> = base_text.chars().collect();
    let reading_chars: Vec<char> = reading.chars().collect();
    let lb = base_chars.len();
    let lr = reading_chars.len();
    let mut ib = lb;
    let mut ir = lr;
    while ib > 0 && ir > 0 {
        ib -= 1;
        ir -= 1;
        if base_chars[ib] != reading_chars[ir] {
            return (ib + 1, ir + 1);
        }
    }
    if lr >= lb {
        (0, lr - lb)
    } else {
        (lb - lr, 0)
    }
}

/// Port of `ichiran/dict:root-diff-fn` (`dict-errata.lisp:104`).
///
/// Returns a closure that rewrites the leading `b` characters of its
/// input with the leading `r` characters of `reading`, where `(b, r)
/// = root_diff(base_text, reading)`.
pub fn root_diff_fn(base_text: &str, reading: &str) -> impl Fn(&str) -> String {
    let (b, r) = root_diff(base_text, reading);
    let prefix: String = reading.chars().take(r).collect();
    move |text| {
        let mut out = prefix.clone();
        out.extend(text.chars().skip(b));
        out
    }
}




































/// Port of `ichiran/dict:*skip-words*` (`dict-errata.lisp:1155`).
///
/// Seqs of words that aren't really words (suffixes, etc.); a candidate
/// whose seq-set intersects this list scores 0 and is dropped.
pub static SKIP_WORDS: &[i32] = &[
    2822120, // ても良い
    2013800, // ちゃう
    2108590, // とく
    2029040, // ば
    2428180, // い
    2654250, // た
    2561100, // うまいな
    2210270, // ませんか
    2210710, // ましょうか
    2257550, // ない
    2210320, // ません
    2017560, // たい
    2394890, // とる
    2194000, // であ
    2568000, // れる/られる
    2537250, // しようとする
    2760890, // 三箱
    2831062, // てる
    2831063, // てく
    2029030, // ものの
    2568020, // せる
    900000,  // たそう
    2827357, // まう
];


/// Port of `ichiran/dict:*final-prt*` (`dict-errata.lisp:1182`).
///
/// Seqs of words that only have meaning when they're the final
/// segment of a path.
pub static FINAL_PRT: &[i32] = &[
    2017770, // かい
    // 1008450 // では (commented out upstream)
    2425930, // なの
    // 2780660 // もの (commented out upstream)
    2130430, // け / っけ
    2029130, // ぞ
    2834812, // ぜ
    2718360, // がな
    2201380, // わい
    2722170, // のう
    2751630, // かいな
];

/// Port of `ichiran/dict:*semi-final-prt*` (`dict-errata.lisp:1196`).
///
/// Particles that are final but also have other uses; the final-prt
/// list plus さ/し/な/ね/わ.
pub fn semi_final_prt() -> &'static [i32] {
    static CACHE: OnceLock<Vec<i32>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut out: Vec<i32> = FINAL_PRT.to_vec();
            out.extend_from_slice(&[
                2029120, // さ
                2086640, // し
                2029110, // な
                2029080, // ね
                2029100, // わ
            ]);
            out
        })
        .as_slice()
}

/// Port of `ichiran/dict:*copulae*` (`dict-errata.lisp:1205`).
///
/// JMdict seqs treated as copulae (e.g. だ) during scoring.
pub static COPULAE: &[i32] = &[
    2089020, // だ
            // 2755350 // じゃない (commented out upstream)
];

/// Port of `ichiran/dict:*non-final-prt*` (`dict-errata.lisp:1209`).
///
/// Particles that don't get the final-position score bonus; the only
/// entry is `ん` (2139720).
pub static NON_FINAL_PRT: &[i32] = &[
    2139720, // ん
];

/// Port of `ichiran/dict:*no-kanji-break-penalty*` (`dict-errata.lisp:1214`).
///
/// Seqs of words that are exempt from the kanji-break penalty.
pub static NO_KANJI_BREAK_PENALTY: &[i32] = &[
    1169870, // 飲む
    1198360, // 会議
    1277450, // 好き
    2028980, // で
    1423000, // 着る
    1164690, // 一段
    1587040, // 言う
    2827864, // なので
];

/// Port of `ichiran/dict:*force-kanji-break*` (`dict-errata.lisp:1226`).
///
/// Literal substrings that force the segmenter to break at a kanji
/// boundary.
pub static FORCE_KANJI_BREAK: &[&str] = &["です"];

/// Port of `ichiran/dict:*no-kanji-break*` (`dict-errata.lisp:1229`).
///
/// Literal substrings that do not cause a kanji break in the segmenter.
pub static NO_KANJI_BREAK: &[&str] = &["日置"];

/// Port of `ichiran/dict:*skip-conj-forms*` (`dict-errata.lisp:1310`).
///
/// Conjugation forms whose hits the segmenter drops.
pub static SKIP_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(10), FormToken::Bool(true), FormToken::Any),
    ConjForm::Triple(
        FormToken::Int(3),
        FormToken::Bool(true),
        FormToken::Bool(true),
    ),
    ConjForm::Quadruple(
        FormToken::Str("vs-s"),
        FormToken::Int(5),
        FormToken::Any,
        FormToken::Any,
    ),
];

/// Port of `ichiran/dict:*weak-conj-forms*` (`dict-errata.lisp:1316`).
///
/// Conjugation forms whose hits the segmenter scores down rather than
/// drops outright (the "weak" tier).
pub static WEAK_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(51), FormToken::Any, FormToken::Any), // +conj-adjective-stem+
    ConjForm::Triple(FormToken::Int(52), FormToken::Any, FormToken::Any), // +conj-negative-stem+
    ConjForm::Triple(FormToken::Int(53), FormToken::Any, FormToken::Any), // +conj-causative-su+
    ConjForm::Triple(FormToken::Int(54), FormToken::Any, FormToken::Any), // +conj-adjective-literary+
    ConjForm::Triple(FormToken::Int(9), FormToken::Bool(true), FormToken::Any),
];

/// Port of `ichiran/dict:errata-conj-description-hook` (`dict-errata.lisp:1242`).
///
/// Adds the five ichiran-internal conjugation types
/// (`+conj-adverbial+`=50 … `+conj-adjective-literary+`=54) to the
/// conj-id → description map after it is loaded from conj.csv.
pub fn errata_conj_description_hook(hash: &mut HashMap<i32, String>) {
    hash.insert(50, "Adverbial".to_string()); // +conj-adverbial+
    hash.insert(51, "Adjective Stem".to_string()); // +conj-adjective-stem+
    hash.insert(52, "Negative Stem".to_string()); // +conj-negative-stem+
    hash.insert(53, "Causative (~su)".to_string()); // +conj-causative-su+
    hash.insert(54, "Old/literary form".to_string()); // +conj-adjective-literary+
}

/// Port of `ichiran/dict:errata-conj-rules-hook` (`dict-errata.lisp:1250`).
///
/// Post-load fixups on the conjugation-rules hash (pos-id → list of
/// `conjugation-rule`): adds adverbial / stem / literary rules for
/// `adj-i` and `adj-ix`, a `v5aru` irregular, patches negative-formal
/// okurigana for `v1`/`v1-s` and the negative-conditional for `v5u`,
/// drops `vs-s` potential forms, and (over every entry) rewrites godan
/// causative-su and adds a negative-stem rule for `v5*`.
pub fn errata_conj_rules_hook(hash: &mut HashMap<i32, Vec<ConjugationRule>>) {
    // dict-errata.lisp:1251 — adj-i: adverbial / adjective-stem / literary
    let pos = get_pos_index("adj-i").expect("adj-i in *pos-index*");
    let rules = [
        ConjugationRule {
            pos,
            conj: 50,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "く".to_string(),
            euphr: String::new(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 51,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: String::new(),
            euphr: String::new(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 54,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "き".to_string(),
            euphr: String::new(),
            euphk: String::new(),
        },
    ];
    for rule in rules {
        hash.entry(pos).or_default().insert(0, rule);
    }

    // dict-errata.lisp:1261 — adj-ix: same as adj-i with euphr "よ"
    let pos = get_pos_index("adj-ix").expect("adj-ix in *pos-index*");
    let rules = [
        ConjugationRule {
            pos,
            conj: 50,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "く".to_string(),
            euphr: "よ".to_string(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 51,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: String::new(),
            euphr: "よ".to_string(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 54,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "き".to_string(),
            euphr: "よ".to_string(),
            euphk: String::new(),
        },
    ];
    for rule in rules {
        hash.entry(pos).or_default().insert(0, rule);
    }

    // dict-errata.lisp:1271 — v5aru irregular
    let pos = get_pos_index("v5aru").expect("v5aru in *pos-index*");
    hash.entry(pos).or_default().insert(
        0,
        ConjugationRule {
            pos,
            conj: 3,
            neg: false,
            fml: false,
            onum: 2,
            stem: 1,
            okuri: "り".to_string(),
            euphr: String::new(),
            euphk: String::new(),
        },
    );

    // dict-errata.lisp:1276 — fix non-past negative formal for v1 v1-s
    let posi = ["v1", "v1-s"].map(|tag| get_pos_index(tag).expect("v1/v1-s in *pos-index*"));
    for pos in posi {
        if let Some(rules) = hash.get_mut(&pos) {
            for rule in rules.iter_mut() {
                if rule.conj == 1 && rule.fml && rule.neg {
                    rule.okuri = "ません".to_string();
                }
            }
        }
    }

    // dict-errata.lisp:1282 — fix incorrect negative conditional of v5u
    let pos = get_pos_index("v5u").expect("v5u in *pos-index*");
    if let Some(rules) = hash.get_mut(&pos) {
        for rule in rules.iter_mut() {
            if rule.conj == 11 && !rule.fml && rule.neg {
                rule.okuri = "わなかったら".to_string();
            }
        }
    }

    // dict-errata.lisp:1287 — remove potential forms of vs-s
    let pos = get_pos_index("vs-s").expect("vs-s in *pos-index*");
    hash.entry(pos).or_default().retain(|r| r.conj != 5);

    // dict-errata.lisp:1290 (maphash) — add conj-negative-stem for godan verbs
    for (key, val) in hash.iter_mut() {
        let pos = get_pos(*key);
        // dict-errata.lisp:1294 — conj 7 / onum 2 → causative-su, onum 1
        for r in val.iter_mut() {
            if r.conj == 7 && r.onum == 2 {
                r.conj = 53;
                r.onum = 1;
            }
        }
        // dict-errata.lisp:1298 (alexandria:starts-with-subseq "v5" pos)
        if pos.is_some_and(|p| p.starts_with("v5")) {
            // dict-errata.lisp:1299 — first non-formal negative (conj 1) rule
            if let Some(mut new_rule) = val.iter().find(|r| r.conj == 1 && r.neg && !r.fml).cloned()
            {
                let len = new_rule.okuri.chars().count();
                if len > 2 {
                    new_rule.conj = 52;
                    new_rule.okuri = new_rule.okuri.chars().take(len - 2).collect();
                    val.insert(0, new_rule);
                }
            }
        }
    }
}

/// Port of `ichiran/dict:skip-by-conj-data` (`dict-errata.lisp:1336`).
///
/// True iff `conj_data` is non-empty and every prop matches
/// [`SKIP_CONJ_FORMS`] (empty list → false).
pub fn skip_by_conj_data(conj_data: &[ConjData]) -> bool {
    !conj_data.is_empty() && conj_data.iter().all(matches)
}

fn matches(cd: &ConjData) -> bool {
    cd.prop
        .as_ref()
        .map(|prop| test_conj_prop(prop, SKIP_CONJ_FORMS))
        .unwrap_or(false)
}

/// Port of `ichiran/dict:test-conj-prop` (`dict-errata.lisp:1336`).
///
/// Predicate: does [`ConjProp`] match any element of `forms`? A
/// 3-element form matches `(conj-type neg fml)`, a 4-element form adds
/// `pos`; a `:any` cell always matches.
pub fn test_conj_prop(prop: &ConjProp, forms: &[ConjForm]) -> bool {
    forms.iter().any(|form| match form {
        ConjForm::Triple(ct, neg, fml) => {
            match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
        ConjForm::Quadruple(pos, ct, neg, fml) => {
            match_pos(*pos, &prop.pos)
                && match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
    })
}

fn match_conj_type(token: FormToken, value: i32) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Int(n) => n == value,
        _ => false,
    }
}

fn match_pos(token: FormToken, value: &str) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Str(s) => s == value,
        _ => false,
    }
}

fn match_bool(token: FormToken, value: Option<bool>) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Bool(b) => value == Some(b),
        FormToken::DbNull => value.is_none(),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
