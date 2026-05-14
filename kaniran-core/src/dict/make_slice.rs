//! Port of `ichiran/dict:make-slice` (`dict.lisp:1009`).
//!
//! Seed value for the reusable string-view handle that
//! [`super::subseq_slice::subseq_slice`] adjusts in place upstream.
//! Upstream returns `(make-array 0 :element-type 'character
//! :displaced-to "")` — a zero-length displaced character vector;
//! callers thread this value through repeated `subseq-slice` calls to
//! avoid allocating a fresh displaced array each iteration.
//!
//! ## Divergences from Lisp
//!
//! Returns `&'static str = ""` instead of a displaced character vector.
//! Rust `&str` is already a zero-cost view (pointer + length); the
//! Lisp allocation-reuse mechanic has no counterpart and the seed
//! value is only ever consumed by [`subseq_slice`], which now returns
//! a fresh `&str` per call. The two values compare equal under Lisp
//! `string=` (length 0) and serve the same callsite role.

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
