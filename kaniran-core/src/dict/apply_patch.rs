//! Port of `ichiran/dict:apply-patch` (`dict-grammar.lisp:435`).
//!
//! Replaces the trailing `removed` chars of `root` with `replacement`
//! (patch = `(replacement, removed)`), used by suffix definers to
//! rewrite a candidate root before re-querying the dictionary. Length
//! is measured in characters, not bytes.

pub fn apply_patch(root: &str, patch: (&str, &str)) -> String {
    let (replacement, removed) = patch;
    let removed_chars = removed.chars().count();
    let root_chars = root.chars().count();
    // dict-grammar.lisp:436 (concatenate 'string (subseq root 0 (- ...)) (car patch))
    // Upstream errors via `subseq`'s end-bound check when removed > root.
    // The SBCL message text is implementation detail — only the
    // panic-on-overflow shape is load-bearing.
    let prefix_chars = root_chars
        .checked_sub(removed_chars)
        .expect("apply-patch: removed length exceeds root length");
    let byte_split = root
        .char_indices()
        .nth(prefix_chars)
        .map(|(b, _)| b)
        .unwrap_or(root.len());
    format!("{}{}", &root[..byte_split], replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL: `(ichiran/dict::apply-patch "なさ" (cons "い" "さ"))` → `"ない"`.
    /// Mirrors the `suffix-sugiru` rewrite of a "なさ" / "無さ" tail.
    #[test]
    fn replaces_trailing_sa_with_i() {
        assert_eq!(apply_patch("なさ", ("い", "さ")), "ない");
    }

    /// REPL: `(ichiran/dict::apply-patch "そ" (cons "う" ""))` → `"そう"`.
    /// Mirrors the `suffix-garu` "そ" → "そう" promotion (empty removed).
    #[test]
    fn empty_removed_appends_replacement() {
        assert_eq!(apply_patch("そ", ("う", "")), "そう");
    }

    /// REPL: `(ichiran/dict::apply-patch "あいうえお" (cons "XX" "えお"))` →
    /// `"あいうXX"` (multi-character removal across multi-byte UTF-8).
    #[test]
    fn multi_char_removal_multi_byte() {
        assert_eq!(apply_patch("あいうえお", ("XX", "えお")), "あいうXX");
    }

    /// REPL: `(ichiran/dict::apply-patch "abc" (cons "" "abc"))` → `""`
    /// (entire root is the removed tail, replacement is empty).
    #[test]
    fn full_removal_empty_replacement_yields_empty() {
        assert_eq!(apply_patch("abc", ("", "abc")), "");
    }

    /// REPL: `(ichiran/dict::apply-patch "abc" (cons "" ""))` → `"abc"`
    /// (no-op patch returns the root unchanged).
    #[test]
    fn empty_patch_returns_root_unchanged() {
        assert_eq!(apply_patch("abc", ("", "")), "abc");
    }

    /// REPL: `(length (ichiran/dict::apply-patch "abc" (cons "" "")))` → `3`.
    /// Length pin to verify char counts match the upstream `simple-array
    /// character (3)` return shape.
    #[test]
    fn output_char_length_matches_upstream() {
        let out = apply_patch("abc", ("", ""));
        assert_eq!(out.chars().count(), 3);
    }

    /// REPL: `(apply-patch "abc" (cons "x" "toolong"))` →
    /// `ERROR: The value -4 is not of type (OR (MOD …) NULL) when
    /// binding SB-IMPL::END`. Pins that the Rust port also rejects
    /// removed > root rather than silently wrapping `usize`. The
    /// upstream message text is SBCL-internal and not asserted.
    #[test]
    #[should_panic]
    fn removed_longer_than_root_panics() {
        let _ = apply_patch("abc", ("x", "toolong"));
    }
}
