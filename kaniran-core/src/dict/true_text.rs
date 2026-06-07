//! Port of `ichiran/dict:true-text` (`dict.lisp:555-558`).
//!
//! Returns the surface text of a reading via [`super::counters::methods::text`],
//! descending through any `proxy-text` wrappers to the underlying
//! source first.

use std::borrow::Cow;

use super::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use super::proxy_text_class::ProxyText;
use super::counters::methods::text;

pub fn true_text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    match obj {
        KaniWordDispatchEnum::Proxy(p) => Cow::Borrowed(unwrap_proxy_chain(p)),
        other => text(other),
    }
}

fn unwrap_proxy_chain(start: &ProxyText) -> &str {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Kanji(k) => return &k.text,
            KaniSimpleTextDispatchEnum::Kana(k) => return &k.text,
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
        }
    }
}
