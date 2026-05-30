//! Port of `ichiran/dict:true-text` (`dict.lisp:555-558`).
//!
//! Returns the surface text of a reading, descending through any
//! `proxy-text` wrappers to the underlying source. The Lisp gf has
//! two methods:
//!
//! ```lisp
//! (defgeneric true-text (obj)
//!   (:method (obj) (text obj))
//!   (:method ((obj proxy-text)) (true-text (source obj))))
//! ```
//!
//! The Rust port dispatches on [`KaniWordDispatchEnum`]. The
//! `proxy-text` branch unwraps through [`ProxyText::source`]
//! iteratively (the source is itself a [`KaniSimpleTextDispatchEnum`]
//! that may carry another `Proxy`); every other branch delegates to
//! [`super::text::text`] so the counter-text concatenation lives in
//! exactly one place.

use std::borrow::Cow;

use super::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use super::proxy_text_class::ProxyText;
use super::text::text;

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
