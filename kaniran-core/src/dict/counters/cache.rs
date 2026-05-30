//! Port of `ichiran/dict:*counter-cache*` (`dict-counters.lisp:221`).
//!
//! Per-text registry of [`CounterArgs`] recipes that `find-counter`
//! (unported) iterates and instantiates per query. Owned by
//! [`KaniranContext::counter_cache`]; built once by
//! [`build_counter_cache`] during `from_url`.
//!
//! Body mirrors `defcache :counters`: empty `""` seed for
//! `number-text`, then `*special-counters*` dispatch (or default
//! counter-text construction over kanji + katakana-kana for foreign
//! seqs), then `:accepts` suffix expansion and `目` ordinal pass.



use crate::characters::char_classes::{CharClass, test_word};
use crate::characters::kana_class::{KanaClass, char_class_hash};
use crate::characters::voicing::{Voicing, geminate, rendaku};
use crate::conn::kani_context::KaniranContext;
use crate::dict::counters::classes::{Counter, CounterSource, DigitOp, DigitOptKey};
use crate::dict::counters::find_counter::{get_counter_readings, get_digit};
use crate::dict::counters::special::special_counters;
use crate::dict::dao::KanaText;
use crate::dict::kani::{CounterArgs, CounterClass, SuffixKind};
use crate::dict::dao::KanjiText;
use crate::numbers::num_class::{DIGIT_TO_KANA, POWER_TO_KANA};
use std::collections::HashMap;

pub type CounterCache = HashMap<String, Vec<CounterArgs>>;

pub fn counter_cache(ctx: &KaniranContext) -> &CounterCache {
    &ctx.counter_cache
}

pub async fn build_counter_cache(ctx: &KaniranContext) -> Result<CounterCache, sqlx::Error> {
    let mut cache: CounterCache = HashMap::new();

    add_args(&mut cache, CounterArgs::new(CounterClass::NumberText, "", ""));

    let readings = get_counter_readings(ctx).await?;
    let specials = special_counters();
    let foreign_set: std::collections::HashSet<i32> = COUNTER_FOREIGN.iter().copied().collect();

    for (seq, (kanji, kana)) in &readings {
        if let Some(special_fn) = specials.get(seq) {
            for entry in special_fn(kanji, kana) {
                add_args(&mut cache, entry);
            }
        } else {
            add_default_entries(&mut cache, *seq, kanji, kana, &foreign_set);
        }
    }

    // Ordinal pass: snapshot keys so the in-loop add_args doesn't
    // perturb iteration over the same map.
    let snapshot: Vec<String> = cache.keys().cloned().collect();
    for counter in snapshot {
        if counter.is_empty() {
            continue;
        }
        if counter.chars().count() > 1 && counter.ends_with('目') {
            continue;
        }
        let cord = format!("{}目", counter);
        if cache.contains_key(&cord) {
            continue;
        }
        let originals: Vec<CounterArgs> = cache.get(&counter).cloned().unwrap_or_default();
        for old in originals {
            if old.ordinalp {
                continue;
            }
            let new_suffix = match old.suffix.as_deref() {
                Some(s) => format!("{}め", s),
                None => "め".to_string(),
            };
            let derivative = CounterArgs {
                text: cord.clone(),
                suffix: Some(new_suffix),
                ordinalp: true,
                ..old
            };
            add_args(&mut cache, derivative);
        }
    }

    Ok(cache)
}

fn add_default_entries(
    cache: &mut CounterCache,
    seq: i32,
    kanji: &[KanjiText],
    kana: &[KanaText],
    foreign_set: &std::collections::HashSet<i32>,
) {
    let foreign = kanji.is_empty() || foreign_set.contains(&seq);
    let kana_first = kana.first().map(|k| k.text.clone()).unwrap_or_default();
    let accepts: Vec<SuffixKind> = COUNTER_ACCEPTS
        .iter()
        .find(|(s, _)| *s == seq)
        .map(|(_, suffixes)| suffixes.to_vec())
        .unwrap_or_default();

    for kt in kanji {
        let entry = build_default_entry(
            &kt.text,
            &kana_first,
            CounterSource::Kanji(kt.clone()),
            foreign,
            accepts.clone(),
        );
        add_args(cache, entry);
    }

    if foreign {
        for kt in kana.iter().filter(|k| test_word(&k.text, CharClass::Katakana)) {
            let entry = build_default_entry(
                &kt.text,
                &kana_first,
                CounterSource::Kana(kt.clone()),
                foreign,
                accepts.clone(),
            );
            add_args(cache, entry);
        }
    }
}

fn build_default_entry(
    text: &str,
    kana_first: &str,
    source: CounterSource,
    foreign: bool,
    accepts: Vec<SuffixKind>,
) -> CounterArgs {
    let ordinalp = text.chars().count() > 1 && text.ends_with('目');
    CounterArgs::new(CounterClass::Text, text, kana_first)
        .source(Some(source))
        .ordinalp(ordinalp)
        .accepts(accepts)
        .foreign(foreign)
}

/// Mirrors `add-args*`. Multi-text was already pre-expanded in
/// [`crate::dict::kani::args_multi`] so the upstream's outer
/// `add-args` list-text branch has nothing to do here.
///
/// `Vec::insert(0, …)` mirrors Lisp's `(push x list)` — newest
/// recipe at index 0. `find-counter` (when ported) iterates in this
/// order, so the most-recently-registered recipe wins the first
/// verify-pass.
fn add_args(cache: &mut CounterCache, entry: CounterArgs) {
    let key = entry.text.clone();
    let accepts = entry.accepts.clone();
    cache.entry(key).or_default().insert(0, entry.clone());

    for suf in accepts {
        let Some((_, suf_text, suf_kana, suf_desc)) =
            COUNTER_SUFFIXES.iter().copied().find(|(s, _, _, _)| *s == suf)
        else {
            continue;
        };
        let new_text = format!("{}{}", entry.text, suf_text);
        let new_suffix = match entry.suffix.as_deref() {
            Some(prefix) => format!("{}{}", prefix, suf_kana),
            None => suf_kana.to_string(),
        };
        let mut new_descriptions = Vec::with_capacity(entry.suffix_descriptions.len() + 1);
        new_descriptions.push(suf_desc.to_string());
        new_descriptions.extend(entry.suffix_descriptions.iter().cloned());

        let derivative = CounterArgs {
            text: new_text.clone(),
            suffix: Some(new_suffix),
            suffix_descriptions: new_descriptions,
            ..entry.clone()
        };
        cache.entry(new_text).or_default().insert(0, derivative);
    }
}

// ----- was counter_join.rs -----

pub fn counter_join(
    counter: &Counter,
    n: i64,
    mut number_kana: String,
    mut counter_kana: String,
) -> String {
    let base = counter.base();
    let digit = get_digit(n);
    // dict-counters.lisp:103 — (gethash (char counter-kana 0) *char-class-hash*)
    let head = counter_kana
        .chars()
        .next()
        .and_then(|c| char_class_hash().get(&c).copied());

    // dict-counters.lisp:104-105 — (assoc digit (digit-opts obj)) /
    // (assoc :off (digit-opts obj))
    let digit_entry = digit.and_then(|d| {
        base.digit_opts
            .iter()
            .find(|e| matches!(e.key, DigitOptKey::Digit(dd) if i64::from(dd) == d))
    });
    let off_present = base
        .digit_opts
        .iter()
        .any(|e| matches!(e.key, DigitOptKey::Off));

    // dict-counters.lisp:106-123 — (when (or off digit-opts) ... loop ...
    // (return-from counter-join (call-next-method obj n number-kana counter-kana)))
    if off_present || digit_entry.is_some() {
        if let Some(entry) = digit_entry {
            let d = digit.expect("digit must be Some when entry matched on Digit(d)");
            let mut mod_counter = false;
            for opt in &entry.ops {
                match opt {
                    DigitOp::Replace(s) => {
                        if mod_counter {
                            counter_kana = s.clone();
                        } else {
                            // dict-counters.lisp:112-116 — splice the
                            // digit/power's own kana stem off the tail
                            // of number-kana and append the override.
                            let stem_chars = digit_kana_char_len(d);
                            let nk_chars: Vec<char> = number_kana.chars().collect();
                            let keep = nk_chars.len().saturating_sub(stem_chars);
                            let mut new_nk: String = nk_chars[..keep].iter().collect();
                            new_nk.push_str(s);
                            number_kana = new_nk;
                        }
                    }
                    DigitOp::Geminate => geminate(&mut number_kana),
                    DigitOp::Rendaku => rendaku(&mut counter_kana, Voicing::Dakuten),
                    DigitOp::Handakuten => rendaku(&mut counter_kana, Voicing::Handakuten),
                    DigitOp::Counter => mod_counter = true,
                }
            }
        }
        // dict-counters.lisp:5-7 — (T T T T) default method: just concat.
        return format!("{}{}", number_kana, counter_kana);
    }

    // dict-counters.lisp:125-146 — (when (counter-foreign obj) ...
    // (return-from counter-join (call-next-method)))
    if base.foreign {
        if let (Some(d), Some(h)) = (digit, head) {
            match d {
                6 if matches!(
                    h,
                    KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                        | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po
                ) => geminate(&mut number_kana),
                8 | 10 if matches!(
                    h,
                    KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                        | KanaClass::Sa | KanaClass::Shi | KanaClass::Su | KanaClass::Se | KanaClass::So
                        | KanaClass::Ta | KanaClass::Chi | KanaClass::Tsu | KanaClass::Te | KanaClass::To
                        | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po
                ) => geminate(&mut number_kana),
                100 if matches!(
                    h,
                    KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                ) => geminate(&mut number_kana),
                _ => {}
            }
        }
        return format!("{}{}", number_kana, counter_kana);
    }

    // dict-counters.lisp:148-200 — standard (case digit ...) over all
    // counter classes that don't have explicit digit-opts.
    if let (Some(d), Some(h)) = (digit, head) {
        match d {
            1 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                | KanaClass::Sa | KanaClass::Shi | KanaClass::Su | KanaClass::Se | KanaClass::So
                | KanaClass::Ta | KanaClass::Chi | KanaClass::Tsu | KanaClass::Te | KanaClass::To => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            3 => {
                if matches!(
                    h,
                    KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho
                ) {
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
            }
            // dict-counters.lisp:160-162 — digit 4 case is `#-(and)`
            // commented out upstream; intentionally a no-op.
            4 => {}
            6 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            8 | 10 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                | KanaClass::Sa | KanaClass::Shi | KanaClass::Su | KanaClass::Se | KanaClass::So
                | KanaClass::Ta | KanaClass::Chi | KanaClass::Tsu | KanaClass::Te | KanaClass::To
                | KanaClass::Pa | KanaClass::Pi | KanaClass::Pu | KanaClass::Pe | KanaClass::Po => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            100 => match h {
                KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            1000 | 10000 => {
                if matches!(
                    h,
                    KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho
                ) {
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
            }
            _ => {}
        }
    }
    format!("{}{}", number_kana, counter_kana)
}

/// Length, in characters, of the kana stem that represents `digit`
/// inside `number_kana`. Mirrors `dict-counters.lisp:112-114`:
///
/// ```text
/// (length (if (< digit 10)
///             (getf *digit-to-kana* digit)
///             (getf *power-to-kana* (round (log digit 10)))))
/// ```
///
/// Lisp `length` on a `simple-string` is character count (= code-point
/// count under SBCL), so use [`str::chars`] / [`Iterator::count`] —
/// not `String::len`, which is byte count and would split multi-byte
/// kana wrong (every entry is in the BMP, 3 bytes per char in UTF-8).
fn digit_kana_char_len(digit: i64) -> usize {
    if digit < 10 {
        DIGIT_TO_KANA[digit as usize].chars().count()
    } else {
        let exp = (digit as f64).log10().round() as u8;
        POWER_TO_KANA
            .iter()
            .find(|(e, _)| *e == exp)
            .map(|(_, s)| s.chars().count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test_counter_join {
    //! Tests cover the per-branch logic that fixture replay alone
    //! wouldn't pin clearly: the `:c` mod-counter sequencing, the
    //! `:off` early-out, the digit-stem replacement char-length math.
    //! Bulk behavioral coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/counter_join.parquet`
    //! (131k rows) — replayed by `audit_fixtures`.
    use super::*;
    use crate::dict::counters::classes::{
        Counter, CounterText, DigitOp, DigitOptEntry, DigitOptKey, Common,
    };

    fn make_counter(digit_opts: Vec<DigitOptEntry>, foreign: bool) -> Counter {
        Counter::Base(CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: "0".into(),
            number: 0,
            source: None,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts,
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign,
        })
    }

    #[test]
    fn off_skips_all_euphony() {
        // dict-counters.lisp:456 — counter-text "立て" :digit-opts ((:off)).
        // For digit 1 + ka-row, the standard rule would geminate; :off
        // suppresses it.
        let c = make_counter(
            vec![DigitOptEntry { key: DigitOptKey::Off, ops: Vec::new() }],
            false,
        );
        let out = counter_join(&c, 1, "いち".into(), "か".into());
        assert_eq!(out, "いちか");
    }

    #[test]
    fn digit_replace_swaps_number_stem() {
        // dict-counters.lisp:386 — counter-text "時" digit-opts ((4 "よ")).
        // For digit 4: number-kana "よん" → strip stem "よん" (len 2),
        // append "よ" → "よ".
        let c = make_counter(
            vec![DigitOptEntry {
                key: DigitOptKey::Digit(4),
                ops: vec![DigitOp::Replace("よ".into())],
            }],
            false,
        );
        let out = counter_join(&c, 4, "よん".into(), "じ".into());
        assert_eq!(out, "よじ");
    }

    #[test]
    fn mod_counter_then_string_replaces_counter_kana() {
        // dict-counters.lisp:425 — counter-text "羽" digit-opts
        // ((3 :c "ば") ...). For digit 3: :c flips mod-counter, then
        // the string replaces counter-kana itself (not number-kana).
        let c = make_counter(
            vec![DigitOptEntry {
                key: DigitOptKey::Digit(3),
                ops: vec![DigitOp::Counter, DigitOp::Replace("ば".into())],
            }],
            false,
        );
        let out = counter_join(&c, 3, "さん".into(), "わ".into());
        assert_eq!(out, "さんば");
    }

    #[test]
    fn power_digit_replace_strips_power_stem() {
        // dict-counters.lisp:471 — counter-text "世紀" digit-opts
        // ((10 "じっ")). For digit 10: number-kana ends in "じゅう"
        // (len 3 chars); strip and append "じっ".
        let c = make_counter(
            vec![DigitOptEntry {
                key: DigitOptKey::Digit(10),
                ops: vec![DigitOp::Replace("じっ".into())],
            }],
            false,
        );
        let out = counter_join(&c, 10, "じゅう".into(), "せいき".into());
        assert_eq!(out, "じっせいき");
    }

    #[test]
    fn foreign_geminates_only_for_specific_heads() {
        // dict-counters.lisp:127-130 — foreign + digit 6 + ka-row: geminate.
        let c = make_counter(Vec::new(), true);
        let out = counter_join(&c, 6, "ろく".into(), "か".into());
        assert_eq!(out, "ろっか");
        // Same setup, t-row head: foreign-digit-6 doesn't fire (only ka/p-row).
        let out2 = counter_join(&c, 6, "ろく".into(), "た".into());
        assert_eq!(out2, "ろくた");
    }

    #[test]
    fn standard_digit_1_geminates_and_handakutens_h_row() {
        // dict-counters.lisp:148-156 — digit 1 + ha-row: geminate
        // number-kana, handakuten counter-kana.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 1, "いち".into(), "ほん".into());
        assert_eq!(out, "いっぽん");
    }

    #[test]
    fn standard_digit_4_is_noop() {
        // dict-counters.lisp:160-162 — digit 4 case is commented out
        // upstream (`#-(and)` reader-conditional). Even h-row stays put.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 4, "よん".into(), "ほん".into());
        assert_eq!(out, "よんほん");
    }

    #[test]
    fn n_zero_no_digit_match_just_concatenates() {
        // get-digit 0 returns nil (n divisible by 10^8); the body's case
        // arms all skip → plain concat of れい + counter-kana.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 0, "れい".into(), "ほん".into());
        assert_eq!(out, "れいほん");
    }

    #[test]
    fn empty_counter_kana_no_head_lookup() {
        // (char counter-kana 0) on "" would error in Lisp — guarded
        // upstream by populator never producing empty counter-kana, but
        // pin the Rust head=None path here so it's not a future panic.
        let c = make_counter(Vec::new(), false);
        let out = counter_join(&c, 1, "いち".into(), String::new());
        assert_eq!(out, "いち");
    }
}


// ----- was _star_counter_suffixes_star_.rs -----

pub static COUNTER_SUFFIXES: &[(SuffixKind, &str, &str, &str)] = &[
    (SuffixKind::Kan, "間", "かん", "[duration]"),
    (SuffixKind::Kango, "間後", "かんご", "[after ...]"),
    (SuffixKind::Chuu, "中", "ちゅう", "[among/out of ...]"),
];


// ----- was _star_counter_accepts_star_.rs -----

pub static COUNTER_ACCEPTS: &[(i32, &[SuffixKind])] = &[
    (1194480, &[SuffixKind::Kan]),
    (1490430, &[SuffixKind::Kan]),
    (1333450, &[SuffixKind::Kan, SuffixKind::Kango]),
];


// ----- was _star_counter_foreign_star_.rs -----
pub static COUNTER_FOREIGN: &[i32] = &[1120410];


// ----- was _star_extra_counter_ids_star_.rs -----
pub static EXTRA_COUNTER_IDS: &[i32] = &[
    1255430, // 月
    1606800, // 割
];


// ----- was _star_skip_counter_ids_star_.rs -----
pub static SKIP_COUNTER_IDS: &[i32] = &[
    2426510, // 一個当り
    2220370, // 歳 （とせ）
    2248360, // 入 （しお）
    2423450, // 差し
    2671670, // 幅 （の）
    2735690, // 種 （くさ）
    2838543, // 杯 （はた）
    // mahjong stuff - need some research on how to say these
    2249290, // 荘
    2833260, // 翻
    2833465, // 萬
    2833466, // 索
    2833467, // 筒
];


