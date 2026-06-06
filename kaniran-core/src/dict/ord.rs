//! Port of `ichiran/dict:ord` — generic function returning the
//! ordinal position of a word inside its dictionary entry. A slot
//! reader on the DAO classes; `counter-text`, `proxy-text`, and
//! `compound-text` override it to recurse via `source`/`primary`.

use crate::dict::counter_text_class::CounterSource;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

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
