//! Port of `ichiran/characters:safe-subseq` (`characters.lisp:359-363`).
//!
//! Bounds-checked substring over *character* positions. Returns `None`
//! on out-of-range or inverted (`start > end`) bounds; `end = None`
//! slices to the end of the string.

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

    /// Slicing covers char-indexed CJK input correctly.
    #[test]
    fn slices_by_character_not_byte() {
        let s = "あいうえお";
        assert_eq!(safe_subseq(s, 1, Some(4)).as_deref(), Some("いうえ"));
        assert_eq!(safe_subseq(s, 0, None).as_deref(), Some("あいうえお"));
    }

    /// Out-of-range `start`, `end`, or `start > end` all return None.
    #[test]
    fn rejects_out_of_range_or_inverted() {
        let s = "abc";
        assert_eq!(safe_subseq(s, 4, None), None);
        assert_eq!(safe_subseq(s, 0, Some(4)), None);
        assert_eq!(safe_subseq(s, 2, Some(1)), None);
    }
}
