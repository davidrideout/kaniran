//! Port of `ichiran/dict:text` — the `text` reader, with an explicit
//! counter-text override at `dict-counters.lisp:58-59`.
//!
//! Returns the bare `text` slot for every word variant except
//! counter-text, which returns the digit prefix concatenated with the
//! counter morpheme (e.g. `"1人"` rather than `"人"`).

use std::borrow::Cow;

use super::kani_word::KaniWordDispatchEnum;

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
