//! Port of `ichiran/characters:*normal-chars*`
//! (`characters.lisp:114-116`).
//!
//! Standard ASCII printables followed by full-width katakana — the
//! target side of the abnormal→normal character map.

use std::sync::OnceLock;

use super::_star_full_width_kana_star_::FULL_WIDTH_KANA;

const ASCII_PREFIX: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~";

pub fn normal_chars() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| format!("{ASCII_PREFIX}{FULL_WIDTH_KANA}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts the concatenated string equals the introspected value.
    #[test]
    fn matches_introspected_value() {
        assert_eq!(
            normal_chars(),
            "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜"
        );
    }
}
