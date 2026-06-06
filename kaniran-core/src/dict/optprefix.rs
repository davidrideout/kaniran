//! Port of `ichiran/dict:optprefix` (`dict-split.lisp:523`).
//!
//! Build a closure that prepends `prefix` to its argument when the
//! argument doesn't already start with `prefix`.

pub fn optprefix(prefix: &str) -> impl Fn(&str) -> String {
    let prefix = prefix.to_string();
    move |txt: &str| {
        if txt.starts_with(&prefix) {
            txt.to_string()
        } else {
            format!("{prefix}{txt}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_has_prefix() {
        assert_eq!(optprefix("い")("いう"), "いう");
    }

    #[test]
    fn missing_prefix_prepended() {
        assert_eq!(optprefix("い")("う"), "いう");
    }

    #[test]
    fn empty_input_takes_prefix() {
        assert_eq!(optprefix("い")(""), "い");
    }
}
