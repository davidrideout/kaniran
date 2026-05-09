//! Port of `ichiran/dict:common` (gf — `dict-counters.lisp:0`).
//!
//! Returns the JMdict commonness rank for a word — `0..n` for ranked
//! entries (lower is more common), the `Null` sentinel for entries
//! marked explicitly non-common, and the recursion-or-zero fallback
//! the counter-text method codifies for synthesized counters. Five
//! method bodies upstream:
//!
//! - **kana-text / kanji-text** — auto-generated `:reader common` slot
//!   accessors. Slot type `(or db-null integer)`; the Rust DAOs store
//!   it as `Option<i32>` where `None` ≡ `db-null` ≡ Lisp `:null`.
//! - **counter-text** (`dict-counters.lisp:75-76`):
//!   `(or (counter-common obj) (if (source obj) (common (source obj)) 0))`
//!   — slot if truthy, else recurse on source, else 0.
//! - **proxy-text** (`dict.lisp:577`): `(common (source obj))`.
//! - **compound-text** (`dict.lisp:624`): `(common (primary obj))`.
//! - **entry** (`dict.lisp:32`): SQL query computing `max(common)`
//!   across the entry's kanji-text/kana-text rows. **Not ported here**
//!   — entry isn't in [`KaniWordDispatchEnum`], every callsite that
//!   has an [`super::entry_dao::Entry`] in hand is statically typed
//!   and will need its own ctx-injected async port when the relevant
//!   wave lands.
//!
//! ## Return type — [`Common`]
//!
//! Reuses the existing 3-variant enum from
//! [`super::counter_text_class`]. The mapping:
//!
//! | upstream value | Rust |
//! |---|---|
//! | integer | [`Common::Score`] |
//! | `:null` | [`Common::Null`] |
//! | `nil` (slot default, sourceless counter case) | [`Common::Score(0)`] |
//!
//! `Common::Inherit` (a Rust-only sentinel meaning "slot wasn't
//! supplied at construction") is never produced by this dispatcher —
//! the counter-text branch resolves Inherit to either source's common
//! or `Score(0)` per the upstream `(or ... 0)` short-circuit. Inherit
//! exists upstream only as the slot's pre-resolution state; once the
//! gf runs, it's already collapsed.

use crate::dict::counter_text_class::{Common, Counter, CounterSource};
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::counter_text_class::{Common, Counter, CounterSource, CounterText};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::proxy_text_class::ProxyText;
    use crate::dict::simple_text_class::SimpleText;

    fn kanji_with_common(c: Option<i32>) -> KanjiText {
        KanjiText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: c, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kana: None, state: SimpleText::default(),
        }
    }

    fn kana_with_common(c: Option<i32>) -> KanaText {
        KanaText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: c, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kanji: None, state: SimpleText::default(),
        }
    }

    fn counter(common_slot: Common, source: Option<CounterSource>) -> Counter {
        Counter::Base(CounterText {
            text: String::new(), kana: String::new(),
            number_text: "0".into(), number: 0,
            source, ordinalp: false, suffix: None,
            accepts_suffixes: Vec::new(), suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(), common: common_slot,
            allowed: Vec::new(), foreign: false,
        })
    }

    #[test]
    fn simple_text_some_score() {
        assert_eq!(
            common(&KaniWordDispatchEnum::Kanji(kanji_with_common(Some(5)))),
            Common::Score(5),
        );
    }

    #[test]
    fn simple_text_none_is_null() {
        // `db-null` upstream → :null → Common::Null in Rust.
        assert_eq!(
            common(&KaniWordDispatchEnum::Kana(kana_with_common(None))),
            Common::Null,
        );
    }

    #[test]
    fn counter_score_short_circuits() {
        let c = counter(Common::Score(7), Some(CounterSource::Kana(kana_with_common(Some(99)))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(7));
    }

    #[test]
    fn counter_explicit_null_short_circuits() {
        // Lisp `(or :null ...)` returns :null — :null is truthy so
        // the `or` does NOT recurse on source.
        let c = counter(Common::Null, Some(CounterSource::Kana(kana_with_common(Some(3)))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Null);
    }

    #[test]
    fn counter_inherit_recurses_on_source() {
        let c = counter(Common::Inherit, Some(CounterSource::Kanji(kanji_with_common(Some(11)))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(11));
    }

    #[test]
    fn counter_inherit_no_source_returns_zero() {
        // dict-counters.lisp:75-76 — `(or nil (if nil ... 0))` → 0.
        let c = counter(Common::Inherit, None);
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(0));
    }

    #[test]
    fn counter_inherit_source_with_db_null() {
        // counter Inherit + source whose common is db-null → Common::Null.
        let c = counter(Common::Inherit, Some(CounterSource::Kana(kana_with_common(None))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Null);
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        let leaf = KaniSimpleTextDispatchEnum::Kanji(kanji_with_common(Some(2)));
        let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(leaf), state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(inner), state: SimpleText::default(),
        };
        assert_eq!(
            common(&KaniWordDispatchEnum::Proxy(outer)),
            Common::Score(2),
        );
    }

    #[test]
    fn compound_returns_primary_common() {
        let primary = Box::new(KaniWordDispatchEnum::Kanji(kanji_with_common(Some(4))));
        let words = vec![
            KaniWordDispatchEnum::Kanji(kanji_with_common(Some(4))),
            KaniWordDispatchEnum::Kana(kana_with_common(Some(99))),
        ];
        let c = CompoundText {
            text: String::new(), kana: String::new(),
            primary, words,
            score_base: None, score_mod: ScoreMod::Single(0),
        };
        // Compound common reads primary, ignores other words.
        assert_eq!(
            common(&KaniWordDispatchEnum::Compound(c)),
            Common::Score(4),
        );
    }
}
