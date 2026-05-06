//! Port of `ichiran/dict:*counter-cache*` (`dict-counters.lisp:221`).
//!
//! Per-text registry of counter-instance recipes, keyed by the
//! counter's surface text (Lisp `equal`, Rust `String` equality).
//! Each value is the list of [`CounterArgs`] recipes the populator
//! has built for that text; `find-counter` (unported) iterates the
//! recipes, instantiates each into a [`Counter`], and runs `verify`.
//!
//! Mirrors the upstream `defcache :counters` body verbatim:
//!
//! 1. Empty entry under `""` for `number-text` (the bare-number
//!    counter that handles "1", "2", "100" etc. with no surface form).
//! 2. For every counter seq returned by [`get_counter_readings`],
//!    either run its registered [`super::_star_special_counters_star_`]
//!    fn or apply the default counter-text construction (one entry
//!    per kanji-text reading, plus katakana kana when the seq is
//!    foreign or has no kanji).
//! 3. For each entry's `:accepts` list, generate suffix derivatives
//!    (text + `間` / `間後` / `中`) keyed by the combined text.
//! 4. Final pass: for every non-ordinal entry whose `text + 目` key
//!    is unused, register an ordinal derivative.
//!
//! ## Storage
//!
//! Owned by [`KaniranContext::counter_cache`]. [`build_counter_cache`]
//! runs the populator once during [`KaniranContext::from_url`].

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
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

/// Borrow the populated cache off the context.
pub fn counter_cache(ctx: &KaniranContext) -> &CounterCache {
    &ctx.counter_cache
}

/// Run the upstream `defcache :counters` body and return the
/// populated cache. Called from [`KaniranContext::from_url`].
pub async fn build_counter_cache(ctx: &KaniranContext) -> Result<CounterCache, sqlx::Error> {
    let mut cache: CounterCache = HashMap::new();

    // (add-args "" 'number-text)
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

    // Ordinal expansion: counter + 目 entries for non-ordinal counters
    // whose ordinal key is unused. Iterate a snapshot of the keys so
    // the in-loop add_args calls don't disturb iteration.
    let snapshot: Vec<String> = cache.keys().cloned().collect();
    for counter in snapshot {
        if counter.is_empty() {
            continue;
        }
        // (and (> (length counter) 1) (alexandria:ends-with #\目 counter))
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

/// Default counter-text construction for a seq that has no entry in
/// `*special-counters*`. Iterates the seq's kanji rows (plus the
/// katakana subset of its kana rows when the seq is foreign or has
/// no kanji) and registers one entry per row.
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

/// Push an entry into the cache under `entry.text`, then expand
/// each `accepts` suffix into a derivative entry under
/// `entry.text + suffix.text`. Mirrors the upstream `add-args*` flet.
///
/// The Lisp's outer `add-args` peeled off the multi-text case before
/// calling `add-args*`; we don't need that branch — multi-text was
/// pre-expanded in
/// [`super::kani_counter_args::args_multi`] when the
/// `*special-counters*` lambdas ran.
///
/// Insertion order matches Lisp's `(push x list)` — newest at index
/// 0. `find-counter` iterates the per-key list in this order, and
/// upstream's reliance on `push` makes the most-recently-registered
/// recipe win the first verify-pass. Lower-priority recipes (e.g.
/// the default counter-text fallback ordered before a special-counter
/// override) end up later in the list.
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
