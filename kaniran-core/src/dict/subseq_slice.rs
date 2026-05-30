//! Port of `ichiran/dict:subseq-slice` (`dict.lisp:1013`).
//!
//! Returns a string view of `s[start..end]` using *character* offsets,
//! mirroring upstream's `subseq` / `length` semantics (CL strings index
//! by code point). Asserts `end >= start` (matching the upstream
//! `(assert (>= end start))`) AND `end <= (length s)` — upstream's
//! `adjust-array` signals `The :DISPLACED-TO array is too small.` on
//! `end > (length str)` or `start > (length str)`; with the
//! `end >= start` guarantee, bounding `end` alone is sufficient.
//!
//! ## Divergences from Lisp
//!
//! - **`slice` parameter accepted but ignored.** Upstream reuses the
//!   passed-in displaced character vector via `adjust-array
//!   :displaced-to`, returning the same array handle each call so a
//!   caller in a tight loop can preallocate one slice and re-aim it.
//!   Rust `&str` is already a fat pointer with no allocation to reuse,
//!   so the Rust port returns a fresh `&str` borrow per call. The
//!   parameter is kept on the signature to match the upstream lambda
//!   list at fixture-replay / audit-signatures boundaries. The
//!   returned `&str`'s lifetime is tied to `s` (not `_slice`), so a
//!   caller cannot accidentally rely on the result aliasing the
//!   passed-in buffer — the borrow checker rules that out at compile
//!   time.
//! - **`end` is `Option<usize>`.** Upstream's `&optional (end (length
//!   str))` becomes `Option<usize>` with `None` resolving to the
//!   character count of `s`, matching the upstream initform.
//! - **Character offsets.** Per CONVENTIONS §4.5: `start` and `end`
//!   are character positions to match Lisp captures; converted to
//!   byte offsets internally before slicing `&str`.
//!
//! ## Performance note
//!
//! Each call walks `s.char_indices()` to resolve character → byte
//! offsets, making the port `O(|s|)` versus the upstream's `O(1)`
//! pointer-math `adjust-array`. The Lisp consumers
//! ([`super::grammar::suffix_rules::parse_suffix_val`] dependents
//! `get-suffix-map` / `get-suffixes` / `find-word-suffix`) call it
//! inside an `O(n²)` substring loop, so the asymptotic shape diverges.
//! Revisit once those consumers are ported (e.g. caching
//! `s.chars().count()` once per outer loop on the caller side).

pub fn subseq_slice<'a>(
    _slice: Option<&str>,
    s: &'a str,
    start: usize,
    end: Option<usize>,
) -> &'a str {
    let total_chars = s.chars().count();
    let end_chars = end.unwrap_or(total_chars);
    assert!(
        end_chars >= start,
        "subseq-slice: end ({}) < start ({})",
        end_chars,
        start,
    );
    // dict.lisp:1016 (adjust-array :displaced-to) — upstream signals
    // ":DISPLACED-TO array is too small" when end > (length str); with
    // end >= start above, this also covers start > (length str).
    assert!(
        end_chars <= total_chars,
        "subseq-slice: end ({}) > (length s) ({})",
        end_chars,
        total_chars,
    );
    let byte_start = nth_char_byte(s, start);
    let byte_end = nth_char_byte(s, end_chars);
    &s[byte_start..byte_end]
}

fn nth_char_byte(s: &str, n: usize) -> usize {
    s.char_indices()
        .nth(n)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL: `(subseq-slice nil "あいうえお" 1 3)` → `"いう"` (length 2).
    /// Pins character-offset semantics across multi-byte UTF-8.
    #[test]
    fn character_offsets_multi_byte() {
        let r = subseq_slice(None, "あいうえお", 1, Some(3));
        assert_eq!(r, "いう");
    }

    /// REPL: `(subseq-slice nil "abcde" 0 5)` → `"abcde"`.
    #[test]
    fn full_range_ascii() {
        let r = subseq_slice(None, "abcde", 0, Some(5));
        assert_eq!(r, "abcde");
    }

    /// REPL: `(subseq-slice nil "abcde" 0)` → `"abcde"` (default end).
    #[test]
    fn end_defaults_to_length() {
        let r = subseq_slice(None, "abcde", 0, None);
        assert_eq!(r, "abcde");
    }

    /// REPL: `(subseq-slice nil "abc" 1)` → `"bc"` (default end past start).
    #[test]
    fn end_default_with_offset_start() {
        let r = subseq_slice(None, "abc", 1, None);
        assert_eq!(r, "bc");
    }

    /// REPL: `(subseq-slice nil "hello" 2 2)` → `""` (start == end).
    #[test]
    fn empty_range_when_start_equals_end() {
        let r = subseq_slice(None, "hello", 2, Some(2));
        assert_eq!(r, "");
    }

    /// REPL: passing in an existing slice returns a view of `s` regardless.
    /// `(let ((s (make-slice))) (subseq-slice s "hello" 1 4))` → `"ell"`.
    #[test]
    fn slice_argument_is_ignored() {
        let seed = super::super::make_slice::make_slice();
        let r = subseq_slice(Some(seed), "hello", 1, Some(4));
        assert_eq!(r, "ell");
    }

    /// REPL: `(subseq-slice nil "hello" 4 2)` → assertion failure
    /// `(>= END START)` (END=2, START=4).
    #[test]
    #[should_panic(expected = "subseq-slice: end (2) < start (4)")]
    fn end_less_than_start_panics() {
        let _ = subseq_slice(None, "hello", 4, Some(2));
    }

    /// REPL: `(subseq-slice nil "hello" 0 10)` →
    /// `ERROR: The :DISPLACED-TO array is too small.`
    #[test]
    #[should_panic(expected = "subseq-slice: end (10) > (length s) (5)")]
    fn end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 0, Some(10));
    }

    /// REPL: `(subseq-slice nil "hello" 2 7)` →
    /// `ERROR: The :DISPLACED-TO array is too small.` (start in range,
    /// end past length).
    #[test]
    #[should_panic(expected = "subseq-slice: end (7) > (length s) (5)")]
    fn start_in_range_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 2, Some(7));
    }

    /// REPL: `(subseq-slice nil "hello" 10 12)` →
    /// `ERROR: The :DISPLACED-TO array is too small.` (both out of
    /// range; rejected via the end-bound check).
    #[test]
    #[should_panic(expected = "subseq-slice: end (12) > (length s) (5)")]
    fn start_and_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 10, Some(12));
    }

    /// REPL: `(subseq-slice nil "hello" 5 5)` → `""` (start == end ==
    /// length is the upper-edge OK case, no error).
    #[test]
    fn start_equal_to_length_at_end_is_ok() {
        let r = subseq_slice(None, "hello", 5, Some(5));
        assert_eq!(r, "");
    }

    /// REPL: `(subseq-slice nil "hello" 0 5)` → `"hello"` (end ==
    /// length is the upper-edge OK case).
    #[test]
    fn end_equal_to_length_is_ok() {
        let r = subseq_slice(None, "hello", 0, Some(5));
        assert_eq!(r, "hello");
    }
}
