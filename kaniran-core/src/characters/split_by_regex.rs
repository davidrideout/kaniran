//! Port of `ichiran/characters:split-by-regex` (`characters.lisp:234-236`).
//!
//! Split `s` by `regex`, interleaving the captured groups with the
//! between-match text and dropping any empty pieces. Mirrors
//! cl-ppcre's `(ppcre:split regex str :with-registers-p t)`.
//!
//! With a regex that has a single outer capture group (the upstream
//! usage in `*basic-split-regex*`), the result alternates
//! "between-match text" with "the matched text" — exactly what
//! `basic-split` relies on for its misc/word classification.

use fancy_regex::Regex;

pub fn split_by_regex(regex: &Regex, s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut last = 0;
    for cap in regex.captures_iter(s) {
        let cap = cap.expect("regex iteration error");
        let m = cap.get(0).expect("capture 0 is the whole match");
        let before = &s[last..m.start()];
        if !before.is_empty() {
            out.push(before.to_string());
        }
        for i in 1..cap.len() {
            if let Some(g) = cap.get(i) {
                let g = g.as_str();
                if !g.is_empty() {
                    out.push(g.to_string());
                }
            }
        }
        last = m.end();
    }
    let after = &s[last..];
    if !after.is_empty() {
        out.push(after.to_string());
    }
    out
}
