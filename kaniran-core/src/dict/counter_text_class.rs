//! Port of `ichiran/dict:counter-text` (`dict-counters.lisp:9`).
//!
//! Base type for every entry the `*counter-cache*` populator stores
//! and that `find-counter` instantiates per-query. Carries the surface
//! forms (`text`, `kana`, `number_text`) plus the metadata that drives
//! `counter-join`'s euphonic transformations (`digit_opts`, `foreign`,
//! `accepts_suffixes`) and `find-counter`'s filtering (`allowed`,
//! `common`).
//!
//! Methods (`verify`, `value-string`, `counter-join`, the
//! `initialize-instance :after` that parses `number_text` into the
//! `number` slot, `text` / `get-kana` / `word-type` / etc.) are NOT
//! ported here — they land alongside `find-counter` in a later wave.
//! This file only mirrors the slot shape so the cache populator can
//! land first.
//!
//! ## Family dispatch
//!
//! Counter dispatch follows CONVENTIONS §4.9: each subclass is a
//! distinct type in its own file, and the [`Counter`] enum below is
//! the dispatcher. The cache populator constructs `Counter::Base(...)`,
//! `Counter::Tsu(...)`, etc.; per-generic dispatch methods on
//! [`Counter`] match-and-delegate to the variant's own method.
//!
//! When the wider word-type dispatch surface lands (the simple-text /
//! proxy-text / compound-text / counter-text cross-family generics:
//! `get-kana`, `word-type`, `common`, `seq`, `ord`, etc.), the future
//! top-level `Word` enum will hold this [`Counter`] as one variant.
//! Inter-family `:around` methods stay local to their family
//! dispatcher (counter-text's get-kana `:around` that appends
//! [`CounterText::suffix`] lives on [`Counter::get_kana`]; simple-text's
//! get-kana `:around` lives on the simple-text family dispatcher).
//!
//! [`Counter::get_kana`]: Counter
//! [`CounterText::suffix`]: CounterText#structfield.suffix
//!
//! Slot-typing divergences from the Lisp:
//! - `foreign`: every caller (`(when (counter-foreign obj) ...)` in
//!   `counter-join`) treats it as a predicate, so [`bool`] per
//!   CONVENTIONS §4.1, even though the populator passes the matching
//!   `seq` integer or `t` upstream.
//! - `common`: the slot accepts `nil` (delegate to source), `:null`
//!   (explicitly null), or an integer score. Modeled as a 3-variant
//!   enum [`Common`] per CONVENTIONS §4.3 rather than collapsing the
//!   sentinel.
//! - `digit_opts`: closed set of tagged ops (`:g`, `:r`, `:h`, `:c`,
//!   plus literal-string replacements) keyed by digit or `:off`,
//!   modeled as enums [`DigitOp`] / [`DigitOptKey`] per §4.3. Slot
//!   docstring lists `:d` (dakuten) but no call site or method handles
//!   it, so it is omitted; add it if the data ever uses it.
//! - `source`: holds whichever JMdict reading row produced this
//!   counter (kanji-text or kana-text), or [`None`] for synthesized
//!   `number_text` rows. Modeled as enum [`CounterSource`] per §4.3.

use crate::dict::counter_age_class::CounterAge;
use crate::dict::counter_days_kun_class::CounterDaysKun;
use crate::dict::counter_days_on_class::CounterDaysOn;
use crate::dict::counter_halfhour_class::CounterHalfhour;
use crate::dict::counter_hifumi_class::CounterHifumi;
use crate::dict::counter_months_class::CounterMonths;
use crate::dict::counter_people_class::CounterPeople;
use crate::dict::counter_tsu_class::CounterTsu;
use crate::dict::counter_wari_class::CounterWari;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::kani_suffix_kind::SuffixKind;
use crate::dict::number_text_class::NumberText;

#[derive(Debug, Clone)]
pub struct CounterText {
    pub text: String,
    pub kana: String,
    pub number_text: String,
    pub number: i32,
    pub source: Option<CounterSource>,
    pub ordinalp: bool,
    pub suffix: Option<String>,
    pub accepts_suffixes: Vec<SuffixKind>,
    pub suffix_descriptions: Vec<String>,
    pub digit_opts: Vec<DigitOptEntry>,
    pub common: Common,
    pub allowed: Vec<i32>,
    pub foreign: bool,
}

#[derive(Debug, Clone)]
pub enum CounterSource {
    Kanji(KanjiText),
    Kana(KanaText),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Common {
    Inherit,
    Null,
    Score(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitOptEntry {
    pub key: DigitOptKey,
    pub ops: Vec<DigitOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitOptKey {
    Off,
    Digit(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigitOp {
    Geminate,
    Rendaku,
    Handakuten,
    Counter,
    Replace(String),
}

/// Family dispatcher for the counter-text class hierarchy. One
/// variant per Lisp class. Per-generic dispatch methods (`get_kana`,
/// `verify`, `value_string`, `counter_join`, etc.) land here when the
/// `find-counter` wave is ported; they match-and-delegate to the
/// variant's own method on the wrapped struct, then apply
/// counter-text's `:around` wrappers (e.g. appending
/// [`CounterText::suffix`] to `get_kana` output) at the dispatcher
/// level. See CONVENTIONS §4.9.
#[derive(Debug, Clone)]
pub enum Counter {
    Base(CounterText),
    NumberText(NumberText),
    Age(CounterAge),
    DaysKun(CounterDaysKun),
    DaysOn(CounterDaysOn),
    Halfhour(CounterHalfhour),
    Hifumi(CounterHifumi),
    Months(CounterMonths),
    People(CounterPeople),
    Tsu(CounterTsu),
    Wari(CounterWari),
}

impl Counter {
    /// Borrow the shared counter-text slot data underlying this
    /// variant. Used by dispatchers and by callers that need to read
    /// inherited slots (`text`, `kana`, `number`, `source`, etc.)
    /// without caring which subclass produced them.
    pub fn base(&self) -> &CounterText {
        match self {
            Counter::Base(c) => c,
            Counter::NumberText(c) => &c.0,
            Counter::Age(c) => &c.0,
            Counter::DaysKun(c) => &c.0,
            Counter::DaysOn(c) => &c.0,
            Counter::Halfhour(c) => &c.0,
            Counter::Hifumi(c) => &c.base,
            Counter::Months(c) => &c.0,
            Counter::People(c) => &c.0,
            Counter::Tsu(c) => &c.0,
            Counter::Wari(c) => &c.0,
        }
    }
}
