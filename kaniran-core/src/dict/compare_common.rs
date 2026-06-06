//! Port of `ichiran/dict:compare-common` (`dict.lisp:1022`).
//!
//! Ranking predicate over two JMdict `common` values that orders
//! readings by commonness (lower rank = more common). Inputs: `None`
//! mirrors Lisp `nil` (no rank), `Some(0)` is the "common but
//! unranked" marker, positive values are rank tiers, and negative or
//! zero c1 values fall off the `cond` ladder and return `Nil`.

/// Faithful image of the three upstream return shapes. Predicate
/// callers consult [`Self::is_truthy`]; fixture replay compares the
/// variant directly to the captured Lisp value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareCommonResult {
    /// Cond fell off without a `t` clause, or a branch evaluated to
    /// `nil` (`(< c1 c2)` returning `nil`, `(and c1 (> c1 0))`
    /// failing, etc.). Maps to Lisp `NIL` / JSON `null`.
    Nil,
    /// First branch — `((not c2) c1)` — fired and returned `c1`
    /// itself (non-`nil` integer). Maps to Lisp integer / JSON
    /// number.
    C1(i64),
    /// Second or third branch returned `T`. Maps to Lisp `T` / JSON
    /// `true`.
    True,
}

impl CompareCommonResult {
    /// Truthiness for the comparator/predicate consumers at
    /// `dict.lisp:867`, `1029`, `1877`. `Nil` is the only falsy
    /// variant — `C1(0)` is truthy because Lisp `0` is truthy.
    pub fn is_truthy(self) -> bool {
        !matches!(self, CompareCommonResult::Nil)
    }
}

pub fn compare_common(c1: Option<i64>, c2: Option<i64>) -> CompareCommonResult {
    // dict.lisp:1023 — ((not c2) c1). Branch returns c1 itself; nil
    // c1 means the branch returns nil.
    if c2.is_none() {
        return match c1 {
            Some(n) => CompareCommonResult::C1(n),
            None => CompareCommonResult::Nil,
        };
    }
    let c2 = c2.unwrap();
    // dict.lisp:1024 — ((= c2 0) (and c1 (> c1 0))). Branch evaluates
    // to T or NIL.
    if c2 == 0 {
        return if matches!(c1, Some(n) if n > 0) {
            CompareCommonResult::True
        } else {
            CompareCommonResult::Nil
        };
    }
    // dict.lisp:1025 — ((and c1 (> c1 0)) (< c1 c2)). Branch only
    // fires when c1 is positive; result is (< c1 c2) which is T or
    // NIL.
    if let Some(n) = c1 {
        if n > 0 {
            return if n < c2 {
                CompareCommonResult::True
            } else {
                CompareCommonResult::Nil
            };
        }
    }
    // cond falls off without a t-clause → nil.
    CompareCommonResult::Nil
}

#[cfg(test)]
mod tests {
    use super::*;
    use CompareCommonResult::*;

    // All assertions REPL-pinned against upstream ichiran. Each value
    // matches the exact Lisp return: branch 1 returns c1 itself, so
    // (compare-common 5 NIL) = 5 (C1(5)); branches 2/3 return T or NIL.
    #[test]
    fn nil_c1_always_nil() {
        // (compare-common NIL <anything>) = NIL.
        for c2 in [None, Some(0), Some(1), Some(2), Some(5), Some(10), Some(-3)] {
            assert_eq!(compare_common(None, c2), Nil);
        }
    }

    #[test]
    fn nil_c2_returns_c1_itself() {
        // (compare-common <integer> NIL) returns c1 (branch 1).
        assert_eq!(compare_common(Some(0), None), C1(0));
        assert_eq!(compare_common(Some(1), None), C1(1));
        assert_eq!(compare_common(Some(2), None), C1(2));
        assert_eq!(compare_common(Some(5), None), C1(5));
        assert_eq!(compare_common(Some(10), None), C1(10));
        assert_eq!(compare_common(Some(-3), None), C1(-3));
    }

    #[test]
    fn zero_c1_only_truthy_when_c2_nil() {
        // (compare-common 0 NIL) = 0 (C1(0), truthy); all others NIL.
        assert_eq!(compare_common(Some(0), None), C1(0));
        assert_eq!(compare_common(Some(0), Some(0)), Nil);
        assert_eq!(compare_common(Some(0), Some(1)), Nil);
        assert_eq!(compare_common(Some(0), Some(2)), Nil);
        assert_eq!(compare_common(Some(0), Some(5)), Nil);
        assert_eq!(compare_common(Some(0), Some(10)), Nil);
        assert_eq!(compare_common(Some(0), Some(-3)), Nil);
    }

    #[test]
    fn c2_zero_returns_true_when_c1_positive() {
        // (compare-common <pos> 0) = T (branch 2); otherwise NIL.
        assert_eq!(compare_common(Some(1), Some(0)), True);
        assert_eq!(compare_common(Some(2), Some(0)), True);
        assert_eq!(compare_common(Some(5), Some(0)), True);
        assert_eq!(compare_common(Some(10), Some(0)), True);
        assert_eq!(compare_common(Some(0), Some(0)), Nil);
        assert_eq!(compare_common(Some(-3), Some(0)), Nil);
    }

    #[test]
    fn positive_pair_lt_predicate() {
        // Branch 3: (compare-common 1 2) = T (since 1 < 2);
        // (compare-common 2 1) = NIL (since 2 not < 1).
        assert_eq!(compare_common(Some(1), Some(2)), True);
        assert_eq!(compare_common(Some(1), Some(5)), True);
        assert_eq!(compare_common(Some(1), Some(10)), True);
        assert_eq!(compare_common(Some(2), Some(5)), True);
        assert_eq!(compare_common(Some(2), Some(10)), True);
        assert_eq!(compare_common(Some(5), Some(10)), True);
        assert_eq!(compare_common(Some(1), Some(1)), Nil);
        assert_eq!(compare_common(Some(2), Some(1)), Nil);
        assert_eq!(compare_common(Some(2), Some(2)), Nil);
        assert_eq!(compare_common(Some(5), Some(1)), Nil);
        assert_eq!(compare_common(Some(5), Some(2)), Nil);
        assert_eq!(compare_common(Some(5), Some(5)), Nil);
        assert_eq!(compare_common(Some(10), Some(1)), Nil);
        assert_eq!(compare_common(Some(10), Some(5)), Nil);
        assert_eq!(compare_common(Some(10), Some(10)), Nil);
    }

    #[test]
    fn negative_c1_falls_off() {
        // (compare-common -3 1) = NIL — c1 not > 0, cond falls off.
        assert_eq!(compare_common(Some(-3), Some(1)), Nil);
        assert_eq!(compare_common(Some(-3), Some(2)), Nil);
        assert_eq!(compare_common(Some(-3), Some(5)), Nil);
        assert_eq!(compare_common(Some(-3), Some(10)), Nil);
        assert_eq!(compare_common(Some(-3), Some(-3)), Nil);
        // (compare-common <any> -3) when c2 != 0: third clause requires
        // c1 > 0, so c1<0 falls off; c1>0 returns (< c1 -3) = NIL for
        // any positive c1.
        assert_eq!(compare_common(Some(1), Some(-3)), Nil);
        assert_eq!(compare_common(Some(2), Some(-3)), Nil);
        assert_eq!(compare_common(Some(5), Some(-3)), Nil);
        assert_eq!(compare_common(Some(10), Some(-3)), Nil);
    }

    #[test]
    fn is_truthy_maps_nil_to_false() {
        assert!(!Nil.is_truthy());
        assert!(C1(0).is_truthy());
        assert!(C1(-3).is_truthy());
        assert!(C1(5).is_truthy());
        assert!(True.is_truthy());
    }
}
