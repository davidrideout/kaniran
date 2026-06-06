//! Port of `ichiran/dict:*length-coeff-sequences*` (`dict.lisp:686`).
//!
//! Per-class coefficient sequences (`:strong`/`:weak`/`:tail`/`:ltail`)
//! that `length-multiplier-coeff` looks up to score a segment by length.

/// Rust-only sidecar tag for the keyword keys in
/// [`LENGTH_COEFF_SEQUENCES`]. Upstream uses bare CL keywords
/// (`:strong`, `:weak`, `:tail`, `:ltail`) inline as `assoc` keys with
/// no named type; the closed `(member …)` ftype declaration at
/// `dict.lisp:693` is the upstream spec for the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KaniLengthClass {
    Strong,
    Weak,
    Tail,
    Ltail,
}

pub static LENGTH_COEFF_SEQUENCES: &[(KaniLengthClass, &[i64])] = &[
    (KaniLengthClass::Strong, &[1, 8, 24, 40, 60]),
    (KaniLengthClass::Weak, &[1, 4, 9, 16, 25, 36]),
    (KaniLengthClass::Tail, &[4, 9, 16, 24]),
    (KaniLengthClass::Ltail, &[4, 12, 18, 24]),
];

#[cfg(test)]
mod tests {
    use super::*;

    // REPL-pinned (.103 SBCL, 2026-05-13):
    //   *length-coeff-sequences* =
    //     ((:STRONG 1 8 24 40 60)
    //      (:WEAK   1 4 9 16 25 36)
    //      (:TAIL   4 9 16 24)
    //      (:LTAIL  4 12 18 24))
    #[test]
    fn matches_introspected_value() {
        assert_eq!(LENGTH_COEFF_SEQUENCES.len(), 4);
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[0],
            (KaniLengthClass::Strong, &[1i64, 8, 24, 40, 60][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[1],
            (KaniLengthClass::Weak, &[1i64, 4, 9, 16, 25, 36][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[2],
            (KaniLengthClass::Tail, &[4i64, 9, 16, 24][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[3],
            (KaniLengthClass::Ltail, &[4i64, 12, 18, 24][..])
        );
    }
}
