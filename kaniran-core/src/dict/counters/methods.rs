use crate::dict::compound_text_class::CompoundText;
use crate::dict::counters::classes::{Common, Counter, CounterSource};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::word_info_class::WordInfoSeq;
use std::borrow::Cow;

/// Port of `ichiran/dict:common` (gf — `dict-counters.lisp:0`).
///
/// Returns the JMdict commonness rank for a word — `0..n` for ranked
/// entries (lower is more common), the `Null` sentinel for entries
/// marked explicitly non-common, and a recursion-or-zero fallback for
/// synthesized counters, dispatching across the simple-text,
/// counter-text, proxy-text, and compound-text word kinds.
pub fn common(obj: &KaniWordDispatchEnum) -> Common {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => simple_common_from_option(k.common),
        KaniWordDispatchEnum::Kana(k) => simple_common_from_option(k.common),
        KaniWordDispatchEnum::Proxy(p) => common_simple(&p.source),
        KaniWordDispatchEnum::Compound(c) => common(&c.primary),
        KaniWordDispatchEnum::Counter(c) => common_counter(c),
    }
}

fn common_simple(obj: &KaniSimpleTextDispatchEnum) -> Common {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => simple_common_from_option(k.common),
        KaniSimpleTextDispatchEnum::Kana(k) => simple_common_from_option(k.common),
        KaniSimpleTextDispatchEnum::Proxy(p) => common_simple(&p.source),
    }
}

fn common_counter(c: &Counter) -> Common {
    // dict-counters.lisp:75-76 — `(or counter-common (if source (common source) 0))`.
    // Inherit / Null are the falsy-in-Lisp values that fall through to
    // the source-or-zero arm; Score is truthy and short-circuits.
    match c.base().common {
        Common::Score(n) => Common::Score(n),
        Common::Null => Common::Null,
        Common::Inherit => match c.base().source.as_ref() {
            Some(s) => common_counter_source(s),
            None => Common::Score(0),
        },
    }
}

fn common_counter_source(s: &CounterSource) -> Common {
    match s {
        CounterSource::Kanji(k) => simple_common_from_option(k.common),
        CounterSource::Kana(k) => simple_common_from_option(k.common),
    }
}

fn simple_common_from_option(v: Option<i32>) -> Common {
    match v {
        Some(n) => Common::Score(n),
        None => Common::Null,
    }
}

/// Port of `ichiran/dict:nokanji` (gf — `dict-counters.lisp:0`).
///
/// Per-reading "kanji-blocked" flag — true when a kana reading should
/// never be paired with a kanji form. A slot on `kanji-text`/`kana-text`;
/// `counter-text` and `proxy-text` recurse via `source`; `compound-text`
/// has no method (returns `None`).
pub fn nokanji(obj: &KaniWordDispatchEnum) -> Option<bool> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Some(k.nokanji),
        KaniWordDispatchEnum::Kana(k) => Some(k.nokanji),
        KaniWordDispatchEnum::Proxy(p) => Some(nokanji_simple(&p.source)),
        KaniWordDispatchEnum::Counter(c) => Some(nokanji_counter(c)),
        KaniWordDispatchEnum::Compound(_) => None,
    }
}

fn nokanji_simple(obj: &KaniSimpleTextDispatchEnum) -> bool {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => k.nokanji,
        KaniSimpleTextDispatchEnum::Kana(k) => k.nokanji,
        KaniSimpleTextDispatchEnum::Proxy(p) => nokanji_simple(&p.source),
    }
}

fn nokanji_counter(c: &Counter) -> bool {
    // dict-counters.lisp:89-90 — `(and (source obj) (nokanji (source obj)))`
    match c.base().source.as_ref() {
        None => false,
        Some(CounterSource::Kanji(k)) => k.nokanji,
        Some(CounterSource::Kana(k)) => k.nokanji,
    }
}

/// Port of `ichiran/dict:ord` — generic function returning the
/// ordinal position of a word inside its dictionary entry. A slot
/// reader on the DAO classes; `counter-text`, `proxy-text`, and
/// `compound-text` override it to recurse via `source`/`primary`.
fn ord_simple(obj: &KaniSimpleTextDispatchEnum) -> i32 {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => k.ord,
        KaniSimpleTextDispatchEnum::Kana(k) => k.ord,
        KaniSimpleTextDispatchEnum::Proxy(p) => ord_simple(&p.source),
    }
}

pub fn ord(obj: &KaniWordDispatchEnum) -> i32 {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => k.ord,
        KaniWordDispatchEnum::Kana(k) => k.ord,
        KaniWordDispatchEnum::Proxy(p) => ord_simple(&p.source),
        KaniWordDispatchEnum::Compound(c) => ord(&c.primary),
        KaniWordDispatchEnum::Counter(c) => match &c.base().source {
            None => 0,
            Some(CounterSource::Kanji(k)) => k.ord,
            Some(CounterSource::Kana(k)) => k.ord,
        },
    }
}

/// Polymorphic dispatcher for the upstream `seq` gf over the
/// [`KaniWordDispatchEnum`] union: the sequence id of a word, recursing
/// through proxy/counter source words and mapping over a compound's
/// constituents.
pub fn seq(obj: &KaniWordDispatchEnum) -> Option<WordInfoSeq> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniWordDispatchEnum::Kana(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniWordDispatchEnum::Proxy(p) => seq_simple(&p.source),
        KaniWordDispatchEnum::Counter(c) => seq_counter(c),
        KaniWordDispatchEnum::Compound(c) => Some(WordInfoSeq::Multi(seq_compound(c))),
    }
}

fn seq_simple(obj: &KaniSimpleTextDispatchEnum) -> Option<WordInfoSeq> {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniSimpleTextDispatchEnum::Kana(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniSimpleTextDispatchEnum::Proxy(p) => seq_simple(&p.source),
    }
}

fn seq_counter(c: &Counter) -> Option<WordInfoSeq> {
    // dict-counters.lisp:79-80 — `(and (source obj) (seq (source obj)))`
    let s = c.base().source.as_ref()?;
    Some(match s {
        CounterSource::Kanji(k) => WordInfoSeq::Single(k.seq),
        CounterSource::Kana(k) => WordInfoSeq::Single(k.seq),
    })
}

fn seq_compound(c: &CompoundText) -> Vec<Option<WordInfoSeq>> {
    // dict.lisp:617 (defmethod seq ((obj compound-text)) (mapcar #'seq (words obj)))
    // — mapcar preserves nil entries; `c.words.iter().map(seq)` mirrors
    // that position-by-position.
    c.words.iter().map(seq).collect()
}

/// Port of `ichiran/dict:source` (gf — `:reader source` on
/// counter-text at `dict-counters.lisp:14` and proxy-text at
/// `dict.lisp:553`).
///
/// Reads the `source` slot of a counter-text or proxy-text word
/// (`None` for words that have no such slot).
/// Borrowed view of whatever a word's `source` slot holds. The two
/// upstream classes that define `source` (counter-text, proxy-text)
/// hold different slot types, so the Rust dispatcher returns a
/// borrowed sum rather than a single owned value.
#[derive(Debug)]
pub enum SourceRef<'a> {
    CounterKanji(&'a KanjiText),
    CounterKana(&'a KanaText),
    ProxySimple(&'a KaniSimpleTextDispatchEnum),
}

pub fn source(obj: &KaniWordDispatchEnum) -> Option<SourceRef<'_>> {
    match obj {
        KaniWordDispatchEnum::Counter(c) => match c.base().source.as_ref()? {
            CounterSource::Kanji(k) => Some(SourceRef::CounterKanji(k)),
            CounterSource::Kana(k) => Some(SourceRef::CounterKana(k)),
        },
        KaniWordDispatchEnum::Proxy(p) => Some(SourceRef::ProxySimple(&p.source)),
        // No method on these classes upstream; callers that statically
        // hold one read the slot directly (or, for kanji/kana, there
        // is no `source` slot at all).
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Compound(_) => None,
    }
}

/// Port of `ichiran/dict:text` — the `text` reader, with an explicit
/// counter-text override at `dict-counters.lisp:58-59`.
///
/// Returns the bare `text` slot for every word variant except
/// counter-text, which returns the digit prefix concatenated with the
/// counter morpheme (e.g. `"1人"` rather than `"人"`).
pub fn text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Cow::Borrowed(&k.text),
        KaniWordDispatchEnum::Kana(k) => Cow::Borrowed(&k.text),
        KaniWordDispatchEnum::Proxy(p) => Cow::Borrowed(&p.text),
        KaniWordDispatchEnum::Compound(c) => Cow::Borrowed(&c.text),
        KaniWordDispatchEnum::Counter(c) => {
            let base = c.base();
            Cow::Owned(format!("{}{}", base.number_text, base.text))
        }
    }
}

/// Port of `ichiran/dict:value-string` (gf — `dict-counters.lisp:44-49`).
///
/// Renders a counter's numeric value as a human-readable display
/// string, dispatching to the counter-text default or a halfhour /
/// months / wari override.
pub fn value_string(counter: &Counter) -> String {
    match counter {
        Counter::Halfhour(c) => c.value_string(),
        Counter::Months(c) => c.value_string(),
        Counter::Wari(c) => c.value_string(),
        // Every other variant inherits the (counter-text) default.
        Counter::Base(c)
        | Counter::NumberText(crate::dict::number_text_class::NumberText(c))
        | Counter::Age(crate::dict::counters::classes::CounterAge(c))
        | Counter::DaysKun(crate::dict::counters::classes::CounterDaysKun(c))
        | Counter::DaysOn(crate::dict::counters::classes::CounterDaysOn(c))
        | Counter::People(crate::dict::counters::classes::CounterPeople(c))
        | Counter::Tsu(crate::dict::counters::classes::CounterTsu(c)) => c.value_string(),
        Counter::Hifumi(c) => c.base.value_string(),
    }
}

/// Port of `ichiran/dict:verify` (`dict-counters.lisp:31`).
///
/// Tells `find-counter` whether a freshly-constructed [`Counter`]
/// recipe is a real match for the queried number string, dispatching
/// to the counter-text default or a tsu / days-on override.
pub fn verify(counter: &Counter, unique: bool) -> bool {
    match counter {
        Counter::Tsu(c) => c.verify(unique),
        Counter::DaysOn(c) => c.verify(unique),
        // Every other variant inherits the (T T) default method.
        Counter::Base(c)
        | Counter::NumberText(crate::dict::number_text_class::NumberText(c))
        | Counter::Age(crate::dict::counters::classes::CounterAge(c))
        | Counter::DaysKun(crate::dict::counters::classes::CounterDaysKun(c))
        | Counter::Halfhour(crate::dict::counters::classes::CounterHalfhour(c))
        | Counter::Months(crate::dict::counters::classes::CounterMonths(c))
        | Counter::People(crate::dict::counters::classes::CounterPeople(c))
        | Counter::Wari(crate::dict::counters::classes::CounterWari(c)) => c.verify(unique),
        Counter::Hifumi(c) => c.base.verify(unique),
    }
}

/// Port of `ichiran/dict:ordinal-str` (`dict-counters.lisp:38`).
///
/// Renders `n` as an English ordinal — "1st", "22nd", "113th", etc.
pub fn ordinal_str(n: i64) -> String {
    let digit = n.rem_euclid(10);
    let teen = (11..=19).contains(&n.rem_euclid(100));
    let suffix = if teen {
        "th"
    } else {
        match digit {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{}{}", n, suffix)
}

/// Port of `ichiran/dict:get-digit` (`dict-counters.lisp:94`).
///
/// Returns the rightmost decimal digit of `n`, or — when that digit
/// is zero — the largest trailing power of ten that divides `n`
/// while its successor in the sequence (10, 100, 1_000, 10_000,
/// 100_000_000) does not. Returns `None` when `n` is divisible by
/// 100_000_000.
pub fn get_digit(n: i64) -> Option<i64> {
    let digit = n % 10;
    if digit != 0 {
        return Some(digit);
    }
    for &(p, pn) in &[
        (10_i64, 100_i64),
        (100, 1_000),
        (1_000, 10_000),
        (10_000, 100_000_000),
    ] {
        if n % pn != 0 {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests;
