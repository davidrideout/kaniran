//! Port of `ichiran/dict:root-diff-fn` (`dict-errata.lisp:104`).
//!
//! Returns a closure that rewrites the leading `b` characters of its
//! input with the leading `r` characters of `reading`, where `(b, r)
//! = root_diff(base_text, reading)`.

use super::root_diff::root_diff;

pub fn root_diff_fn(base_text: &str, reading: &str) -> impl Fn(&str) -> String {
    let (b, r) = root_diff(base_text, reading);
    let prefix: String = reading.chars().take(r).collect();
    move |text| {
        let mut out = prefix.clone();
        out.extend(text.chars().skip(b));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_diff_fn_fixtures() {
        // REPL (.103, `ichiran/dict::root-diff-fn`), 2026-05-31.
        let f1 = root_diff_fn("尋ねる", "たずねる");
        assert_eq!(f1("訪ねる"), "たずねる");
        assert_eq!(f1("尋ねた"), "たずねた");
        let f2 = root_diff_fn("食べる", "たべる");
        assert_eq!(f2("食べた"), "たべた");
        let f3 = root_diff_fn("猫", "猫");
        assert_eq!(f3("犬"), "犬");
        let f4 = root_diff_fn("ね", "たずね");
        assert_eq!(f4("ね"), "たずね");
        assert_eq!(f4("ねた"), "たずねた");
    }
}
