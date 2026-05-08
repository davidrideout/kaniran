//! Port of `ichiran/dict:ord` — generic function returning the
//! ordinal position of a word inside its dictionary entry. Mostly
//! auto-generated `:reader ord` slot accessors with three explicit
//! overrides on the non-DAO classes:
//!
//! ```lisp
//! ;; dict-counters.lisp:82
//! (defmethod ord ((obj counter-text))
//!   (if (source obj) (ord (source obj)) 0))
//!
//! ;; dict.lisp:580
//! (defmethod ord ((obj proxy-text))
//!   (ord (source obj)))
//!
//! ;; dict.lisp:627
//! (defmethod ord ((obj compound-text))
//!   (ord (primary obj)))
//! ```
//!
//! Polymorphic surface: three callsites exercise the gf —
//! `dict.lisp:802` inside `calc-score` (`for ord = (ord reading)`
//! where `reading` is the loop's word-shaped binding),
//! `dict.lisp:861` in calc-score's conj-of branch (`(ord ot)` where
//! `ot` is from `get-original-text` — typically a kanji-text /
//! kana-text DAO), and `dict.lisp:190` in `gloss`'s `print-object`
//! (`(ord obj)` on the locally-known gloss instance). Only the
//! first is genuinely polymorphic across the word union; the other
//! two have locally-known DAO types and translate to direct `.ord`
//! field reads in the Rust port. This dispatcher therefore covers
//! [`KaniWordDispatchEnum`] only; `sense`, `gloss`, and `sense-prop`
//! access their `.ord` field directly at their (locally-typed)
//! callsites — same precedent as [`super::text`].

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
