use crate::conn::kani_backend::KaniBackend;
use crate::characters::char_class::{test_word, CharClass};
use crate::characters::helpers::char_class_hash;
use crate::characters::kani_kana_class::KanaClass;
use crate::characters::voicing::{geminate, rendaku, Voicing};
use crate::conn::kani_context::KaniranContext;
use crate::dict::counters::classes::{Counter, CounterSource, DigitOp, DigitOptKey};
use crate::dict::counters::constants::{
    special_counters, COUNTER_ACCEPTS, COUNTER_FOREIGN, COUNTER_SUFFIXES, EXTRA_COUNTER_IDS,
    SKIP_COUNTER_IDS,
};
use crate::dict::counters::kani_counter_args::{CounterArgs, CounterClass};
use crate::dict::counters::methods::{get_digit, verify};
use crate::dict::dao::KanaText;
use crate::dict::grammar::suffix::kani_suffix_kind::SuffixKind;
use crate::dict::dao::KanjiText;
use crate::numbers::constants::{DIGIT_TO_KANA, POWER_TO_KANA};
use std::collections::{HashMap, HashSet};

/// Port of `ichiran/dict:*counter-cache*` (`dict-counters.lisp:221`).
///
/// Per-text registry of [`CounterArgs`] recipes used to instantiate
/// counter words.
pub type CounterCache = HashMap<String, Vec<CounterArgs>>;

pub fn counter_cache(ctx: &KaniranContext) -> &CounterCache {
    &ctx.counter_cache
}

pub async fn build_counter_cache(ctx: &KaniranContext) -> Result<CounterCache, sqlx::Error> {
    let mut cache: CounterCache = HashMap::new();

    add_args(
        &mut cache,
        CounterArgs::new(CounterClass::NumberText, "", ""),
    );

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
        for kt in kana
            .iter()
            .filter(|k| test_word(&k.text, CharClass::Katakana))
        {
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
/// [`crate::dict::counters::kani_counter_args::args_multi`] so the upstream's outer
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
        let Some((_, suf_text, suf_kana, suf_desc)) = COUNTER_SUFFIXES
            .iter()
            .copied()
            .find(|(s, _, _, _)| *s == suf)
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

/// Port of `ichiran/dict:counter-join` (`dict-counters.lisp:3-7,
/// 101-201`).
///
/// Constructs the kana surface form of a counter expression by
/// splicing `number_kana` and `counter_kana` and applying euphonic
/// transformations (gemination, rendaku, handakuten) keyed by the
/// decimal "digit" (`get-digit n`) and the kana class of
/// `counter_kana`'s first glyph. Three alternative paths fire per
/// counter: per-digit overrides, a foreign (katakana) counter rule,
/// and the standard `case digit` block covering digits 1-10000.
pub fn counter_join(
    counter: &Counter,
    n: u128,
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
            .find(|e| matches!(e.key, DigitOptKey::Digit(dd) if i128::from(dd) == d as i128))
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
                    KanaClass::Ka
                        | KanaClass::Ki
                        | KanaClass::Ku
                        | KanaClass::Ke
                        | KanaClass::Ko
                        | KanaClass::Pa
                        | KanaClass::Pi
                        | KanaClass::Pu
                        | KanaClass::Pe
                        | KanaClass::Po
                ) =>
                {
                    geminate(&mut number_kana)
                }
                8 | 10
                    if matches!(
                        h,
                        KanaClass::Ka
                            | KanaClass::Ki
                            | KanaClass::Ku
                            | KanaClass::Ke
                            | KanaClass::Ko
                            | KanaClass::Sa
                            | KanaClass::Shi
                            | KanaClass::Su
                            | KanaClass::Se
                            | KanaClass::So
                            | KanaClass::Ta
                            | KanaClass::Chi
                            | KanaClass::Tsu
                            | KanaClass::Te
                            | KanaClass::To
                            | KanaClass::Pa
                            | KanaClass::Pi
                            | KanaClass::Pu
                            | KanaClass::Pe
                            | KanaClass::Po
                    ) =>
                {
                    geminate(&mut number_kana)
                }
                100 if matches!(
                    h,
                    KanaClass::Ka | KanaClass::Ki | KanaClass::Ku | KanaClass::Ke | KanaClass::Ko
                ) =>
                {
                    geminate(&mut number_kana)
                }
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
                KanaClass::Ka
                | KanaClass::Ki
                | KanaClass::Ku
                | KanaClass::Ke
                | KanaClass::Ko
                | KanaClass::Sa
                | KanaClass::Shi
                | KanaClass::Su
                | KanaClass::Se
                | KanaClass::So
                | KanaClass::Ta
                | KanaClass::Chi
                | KanaClass::Tsu
                | KanaClass::Te
                | KanaClass::To => {
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
                KanaClass::Ka
                | KanaClass::Ki
                | KanaClass::Ku
                | KanaClass::Ke
                | KanaClass::Ko
                | KanaClass::Pa
                | KanaClass::Pi
                | KanaClass::Pu
                | KanaClass::Pe
                | KanaClass::Po => {
                    geminate(&mut number_kana);
                }
                KanaClass::Ha | KanaClass::Hi | KanaClass::Fu | KanaClass::He | KanaClass::Ho => {
                    geminate(&mut number_kana);
                    rendaku(&mut counter_kana, Voicing::Handakuten);
                }
                _ => {}
            },
            8 | 10 => match h {
                KanaClass::Ka
                | KanaClass::Ki
                | KanaClass::Ku
                | KanaClass::Ke
                | KanaClass::Ko
                | KanaClass::Sa
                | KanaClass::Shi
                | KanaClass::Su
                | KanaClass::Se
                | KanaClass::So
                | KanaClass::Ta
                | KanaClass::Chi
                | KanaClass::Tsu
                | KanaClass::Te
                | KanaClass::To
                | KanaClass::Pa
                | KanaClass::Pi
                | KanaClass::Pu
                | KanaClass::Pe
                | KanaClass::Po => {
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
fn digit_kana_char_len(digit: u128) -> usize {
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

/// Port of `ichiran/dict:find-counter` (`dict-counters.lisp:273`).
///
/// Looks up the recipes registered for `counter` in the counter
/// cache, materializes a [`Counter`] from each recipe with the given
/// `number` text, and keeps the ones [`verify`] accepts. Drops recipes
/// whose [`Counter::new`] raises `NotANumber`.
pub fn find_counter(
    ctx: &KaniranContext,
    number: &str,
    counter: &str,
    unique: Option<bool>,
) -> Vec<Counter> {
    // dict-counters.lisp:273 — `&key (unique t)`. `None` here means
    // "caller didn't supply :UNIQUE", which Lisp resolves to `t`.
    let unique = unique.unwrap_or(true);
    let Some(args_list) = counter_cache(ctx).get(counter) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(args_list.len());
    for args in args_list {
        match Counter::new(args, number) {
            Ok(c) if verify(&c, unique) => out.push(c),
            _ => {}
        }
    }
    out
}

/// Port of `ichiran/dict:get-counter-ids` (`dict-counters.lisp:283`).
///
/// Returns the sorted list of JMdict sequence numbers tagged
/// `pos=ctr` (counter words) on at least one of their senses.
pub async fn get_counter_ids(ctx: &KaniranContext) -> Result<Vec<i32>, sqlx::Error> {
    let mut seqs: Vec<i32> = ctx.store.counter_seqs().await?;
    seqs.sort();
    Ok(seqs)
}

/// Port of `ichiran/dict:get-counter-stags` (`dict-counters.lisp:291`).
///
/// For a set of JMdict sequence numbers, returns two maps —
/// `(stagks, stagrs)` — listing the kanji-restriction and
/// kana-restriction texts attached to any sense whose `pos` is `ctr`.
/// Seqs with no restrictions are absent from the maps.
pub type CounterStags = (HashMap<i32, Vec<String>>, HashMap<i32, Vec<String>>);

pub async fn get_counter_stags(
    ctx: &KaniranContext,
    seqs: &[i32],
) -> Result<CounterStags, sqlx::Error> {
    let mut stagks: HashMap<i32, Vec<String>> = HashMap::new();
    let mut stagrs: HashMap<i32, Vec<String>> = HashMap::new();

    for (seq, text) in ctx.store.counter_stag_rows("stagk", seqs).await? {
        stagks.entry(seq).or_default().push(text);
    }

    for (seq, text) in ctx.store.counter_stag_rows("stagr", seqs).await? {
        stagrs.entry(seq).or_default().push(text);
    }

    Ok((stagks, stagrs))
}

/// Port of `ichiran/dict:get-counter-readings` (`dict-counters.lisp:332`).
///
/// Builds the `(kanji-rows, kana-rows)` reading lists for every counter
/// seq the cache will populate. Per seq, kanji rows whose `text` is
/// missing from the seq's `stagk` restriction list are dropped (kana
/// rows the same way against `stagr`); survivors sort by `ord`.
pub type CounterReadings = HashMap<i32, (Vec<KanjiText>, Vec<KanaText>)>;

pub async fn get_counter_readings(ctx: &KaniranContext) -> Result<CounterReadings, sqlx::Error> {
    let mut counter_ids: Vec<i32> = get_counter_ids(ctx).await?;
    counter_ids.extend(EXTRA_COUNTER_IDS.iter().copied());
    let skip: HashSet<i32> = SKIP_COUNTER_IDS.iter().copied().collect();
    counter_ids.retain(|id| !skip.contains(id));

    let stags = get_counter_stags(ctx, &counter_ids).await?;

    let kanji_readings: Vec<KanjiText> = ctx.store.kanji_texts_by_seq_any(&counter_ids).await?;

    let kana_readings: Vec<KanaText> = ctx.store.kana_texts_by_seq_any(&counter_ids).await?;

    let mut hash: CounterReadings = HashMap::new();

    for r in kanji_readings {
        let stagks = stags.0.get(&r.seq);
        let admit = match stagks {
            None => true,
            Some(list) => list.iter().any(|t| t == &r.text),
        };
        if admit {
            hash.entry(r.seq)
                .or_insert_with(|| (Vec::new(), Vec::new()))
                .0
                .push(r);
        }
    }

    for r in kana_readings {
        let stagrs = stags.1.get(&r.seq);
        let admit = match stagrs {
            None => true,
            Some(list) => list.iter().any(|t| t == &r.text),
        };
        if admit {
            hash.entry(r.seq)
                .or_insert_with(|| (Vec::new(), Vec::new()))
                .1
                .push(r);
        }
    }

    for (_, (kanji, kana)) in hash.iter_mut() {
        kanji.sort_by_key(|r| r.ord);
        kana.sort_by_key(|r| r.ord);
    }

    Ok(hash)
}

#[cfg(test)]
mod tests;
