//! Port of `ichiran/characters:safe-subseq` (`characters.lisp:359-363`).
//!
//! Bounds-checked substring. Returns `None` when `start` or `end` would
//! fall outside `[0, char-length(s)]`, or when `start > end`. Otherwise
//! returns `Some(<chars start..end>)`. With `end = None`, slices to the
//! end of the string.
//!
//! Positions are *character* offsets (CONVENTIONS §4.5) — matches the
//! Lisp's `subseq` semantics on SBCL strings.

pub fn safe_subseq(s: &str, start: usize, end: Option<usize>) -> Option<String> {
    let len = s.chars().count();
    if start > len {
        return None;
    }
    let stop = match end {
        Some(e) if e > len || start > e => return None,
        Some(e) => e,
        None => len,
    };
    Some(s.chars().skip(start).take(stop - start).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slicing covers char-indexed CJK input correctly — without
    /// pinning this, an accidental byte-index implementation passes
    /// every ASCII test.
    #[test]
    fn slices_by_character_not_byte() {
        let s = "あいうえお";
        assert_eq!(safe_subseq(s, 1, Some(4)).as_deref(), Some("いうえ"));
        assert_eq!(safe_subseq(s, 0, None).as_deref(), Some("あいうえお"));
    }

    /// Out-of-range `start`, `end`, or `start > end` all return None,
    /// mirroring the Lisp's `(when ...)` guard.
    #[test]
    fn rejects_out_of_range_or_inverted() {
        let s = "abc";
        assert_eq!(safe_subseq(s, 4, None), None);
        assert_eq!(safe_subseq(s, 0, Some(4)), None);
        assert_eq!(safe_subseq(s, 2, Some(1)), None);
    }
}
