//! Port of `ichiran/characters:join` (`characters.lisp:365-370`).
//!
//! Concatenate `items` with `separator` between each pair. The Lisp
//! `&key key` parameter is dropped — its single upstream caller
//! (`numbers.lisp:136`) pre-maps already, and Rust callers that need
//! per-element transformation can `.iter().map(...).collect::<Vec<_>>()`
//! before passing in. The generic bound lets callers pass `&[&str]`,
//! `&[String]`, etc. interchangeably.

pub fn join<S: AsRef<str>>(separator: &str, items: &[S]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(separator);
        }
        out.push_str(item.as_ref());
    }
    out
}
