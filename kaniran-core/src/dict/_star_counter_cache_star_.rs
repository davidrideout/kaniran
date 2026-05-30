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

use crate::characters::char_classes::CharClass;
use crate::characters::char_classes::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_counter_accepts_star_::COUNTER_ACCEPTS;
use crate::dict::_star_counter_foreign_star_::COUNTER_FOREIGN;
use crate::dict::_star_counter_suffixes_star_::COUNTER_SUFFIXES;
use crate::dict::_star_special_counters_star_::special_counters;
use crate::dict::counter_text_class::CounterSource;
use crate::dict::get_counter_readings::get_counter_readings;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_counter_args::{CounterArgs, CounterClass};
use crate::dict::kani_suffix_kind::SuffixKind;
use crate::dict::kanji_text_dao::KanjiText;
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
/// [`super::kani_counter_args::args_multi`] so the upstream's outer
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
