//! Port of `ichiran/dict:make-slice` (`dict.lisp:1009`).
//!
//! Returns the empty seed string-view that callers thread through
//! `subseq_slice` (upstream a zero-length displaced character vector).

pub fn make_slice() -> &'static str {
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL: `(length (make-slice))` → 0, `(string= (make-slice) "")` → T
    #[test]
    fn empty_seed() {
        let s = make_slice();
        assert_eq!(s.len(), 0);
        assert_eq!(s, "");
    }
}
