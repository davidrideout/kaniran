//! Port of `ichiran/characters:join` (`characters.lisp:365-370`).
//!
//! Concatenate `items` with `separator` between each pair.

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
