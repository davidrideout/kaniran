//! Rust-only sidecar: the keyword-arg recipe that the
//! `*counter-cache*` populator stores per text key, and that
//! `find-counter` later applies to construct a
//! [`crate::dict::counters::classes::Counter`], materialized from the
//! Lisp `def-special-counter` arglists as a typed [`CounterArgs`]
//! struct.

use crate::dict::counters::classes::{Common, CounterSource, DigitOp, DigitOptEntry, DigitOptKey};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_suffix_kind::SuffixKind;
use crate::dict::kanji_text_dao::KanjiText;

/// Tag-only twin of [`crate::dict::counters::classes::Counter`].
/// Separate so [`CounterArgs`] can stay `Clone` without forcing it
/// onto every variant struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterClass {
    Text,
    NumberText,
    Halfhour,
    Hifumi,
    Tsu,
    Wari,
    Age,
    DaysKun,
    DaysOn,
    Months,
    People,
}

#[derive(Debug, Clone)]
pub struct CounterArgs {
    pub class: CounterClass,
    pub text: String,
    pub kana: String,
    pub source: Option<CounterSource>,
    pub digit_opts: Vec<DigitOptEntry>,
    pub digit_set: Vec<i32>,
    pub allowed: Vec<i32>,
    pub foreign: bool,
    pub common: Common,
    pub accepts: Vec<SuffixKind>,
    pub suffix_descriptions: Vec<String>,
    pub ordinalp: bool,
    pub suffix: Option<String>,
}

impl CounterArgs {
    pub fn new(class: CounterClass, text: impl Into<String>, kana: impl Into<String>) -> Self {
        CounterArgs {
            class,
            text: text.into(),
            kana: kana.into(),
            source: None,
            digit_opts: Vec::new(),
            digit_set: Vec::new(),
            allowed: Vec::new(),
            foreign: false,
            common: Common::Inherit,
            accepts: Vec::new(),
            suffix_descriptions: Vec::new(),
            ordinalp: false,
            suffix: None,
        }
    }

    pub fn source(mut self, source: Option<CounterSource>) -> Self {
        self.source = source;
        self
    }

    pub fn digit_opts(mut self, opts: Vec<DigitOptEntry>) -> Self {
        self.digit_opts = opts;
        self
    }

    pub fn digit_set(mut self, set: Vec<i32>) -> Self {
        self.digit_set = set;
        self
    }

    pub fn allowed(mut self, allowed: Vec<i32>) -> Self {
        self.allowed = allowed;
        self
    }

    pub fn foreign(mut self, foreign: bool) -> Self {
        self.foreign = foreign;
        self
    }

    pub fn common(mut self, common: Common) -> Self {
        self.common = common;
        self
    }

    pub fn accepts(mut self, accepts: Vec<SuffixKind>) -> Self {
        self.accepts = accepts;
        self
    }

    pub fn suffix_descriptions(mut self, descriptions: Vec<String>) -> Self {
        self.suffix_descriptions = descriptions;
        self
    }

    pub fn ordinalp(mut self, ordinalp: bool) -> Self {
        self.ordinalp = ordinalp;
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }
}

/// Mirrors the Lisp `(find query readings :key 'text :test 'equal)`.
pub fn find_source(query: &str, kanji: &[KanjiText], kana: &[KanaText]) -> Option<CounterSource> {
    if let Some(r) = kanji.iter().find(|r| r.text == query) {
        return Some(CounterSource::Kanji(r.clone()));
    }
    if let Some(r) = kana.iter().find(|r| r.text == query) {
        return Some(CounterSource::Kana(r.clone()));
    }
    None
}

/// Single-text entry. Mirrors `(args class text kana ...)`.
pub fn args(
    class: CounterClass,
    text: &str,
    kana: &str,
    kanji: &[KanjiText],
    kana_rows: &[KanaText],
) -> CounterArgs {
    CounterArgs::new(class, text, kana).source(find_source(text, kanji, kana_rows))
}

/// Mirrors `(args class '(t1 t2 ...) kana ...)`. Eager per-text
/// expansion — see module doc for the upstream divergence.
pub fn args_multi(
    class: CounterClass,
    texts: &[&str],
    kana: &str,
    kanji: &[KanjiText],
    kana_rows: &[KanaText],
) -> Vec<CounterArgs> {
    texts
        .iter()
        .map(|t| CounterArgs::new(class, *t, kana).source(find_source(t, kanji, kana_rows)))
        .collect()
}

/// Mirrors `(args-suffix class '(stem suf) '(kana-stem kana-suf) ...)`.
/// Cache key = stem + suf concatenated; `:kana` = stem kana; `:suffix`
/// = suf kana; `:source` = stem's row.
pub fn args_suffix(
    class: CounterClass,
    text_parts: (&str, &str),
    kana_parts: (&str, &str),
    kanji: &[KanjiText],
    kana_rows: &[KanaText],
) -> CounterArgs {
    let (stem, suf) = text_parts;
    let (kana_stem, kana_suf) = kana_parts;
    let combined = format!("{}{}", stem, suf);
    CounterArgs::new(class, combined, kana_stem)
        .suffix(kana_suf)
        .source(find_source(stem, kanji, kana_rows))
}

/// Mirrors the Lisp shape `'((3 :r) (4 :h "よ"))` at the callsite.
pub fn digit_opts(items: &[(DigitOptKey, &[DigitOp])]) -> Vec<DigitOptEntry> {
    items
        .iter()
        .map(|(k, ops)| DigitOptEntry { key: *k, ops: ops.to_vec() })
        .collect()
}
