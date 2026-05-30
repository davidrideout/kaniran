//! Port of `ichiran/characters:*normal-chars*`
//! (`characters.lisp:114-116`).
//!
//! Target side of the abnormal→normal character map: standard ASCII
//! printables and full-width katakana, paired index-by-index with
//! [`ABNORMAL_CHARS`][super::constants::ABNORMAL_CHARS].
//!
//! Upstream constructs this as
//! `(concatenate 'string "<85-char ASCII prefix>" *full-width-kana*)`.
//! We mirror the same shape: derive lazily via `OnceLock` from
//! [`FULL_WIDTH_KANA`] plus the inline ASCII prefix. The prefix has no
//! upstream constant — it appears as an inline string literal in the
//! `concatenate` call — so it is hand-typed in this file.
//!
//! Same derivation pattern as `*basic-split-regex*`. A regression test
//! pins the build output to the value the introspector captured so any
//! drift in either input is caught at test time.

use std::sync::OnceLock;

use super::constants::FULL_WIDTH_KANA;

const ASCII_PREFIX: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~";

static CACHE: OnceLock<String> = OnceLock::new();

pub fn normal_chars() -> &'static str {
    CACHE.get_or_init(|| format!("{ASCII_PREFIX}{FULL_WIDTH_KANA}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pinned to the value the Lisp introspector captured. Guards
    /// against drift if the ASCII prefix or `FULL_WIDTH_KANA` changes
    /// without an intentional update here.
    #[test]
    fn matches_introspected_value() {
        assert_eq!(
            normal_chars(),
            "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜"
        );
    }
}
