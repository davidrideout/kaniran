//! Port of `ichiran/dict:*disable-hints*` (`dict.lisp:78`).
//!
//! Re-entrance guard for hint computation. The Lisp `get-kana :around`
//! method on `simple-text` rebinds this to `t` while it computes a
//! hinted form, so the recursive call inside skips the hint branch.
//! `check-easy-hints` (test helper) likewise binds it to `t` to disable
//! hints during a sanity scan.
//!
//! ## Divergence
//!
//! The Lisp `defparameter` is dynamically rebound by `let`. Rust has no
//! native dynamic binding, so this is modelled as a thread-local boolean
//! plus a scoped RAII guard ([`with_disable_hints`]). Same observable
//! effect (the rebinding is visible to inner calls on the same thread,
//! restored on guard drop).
//!
//! ## TODO: prime candidate for restructuring
//!
//! Once `get-kana` and its callers are ported, revisit this. Better
//! Rust shapes:
//!
//! 1. Pass `disable_hints: bool` as an explicit parameter to `get_kana`
//!    so the around-method's recursion just passes `true`. There are
//!    only two read sites (`get-kana :around`, `check-easy-hints`) — a
//!    parameter beats the global.
//! 2. Restructure so the around-method calls a non-around inner fn
//!    directly and the flag isn't needed at all.
//!
//! Either eliminates the thread-local + guard. Kept as-is for now to
//! match the upstream shape until the dependent code lands.

use std::cell::Cell;

thread_local! {
    static DISABLE_HINTS: Cell<bool> = const { Cell::new(false) };
}

pub fn disable_hints() -> bool {
    DISABLE_HINTS.with(|c| c.get())
}

pub struct DisableHintsGuard {
    prev: bool,
}

impl Drop for DisableHintsGuard {
    fn drop(&mut self) {
        let prev = self.prev;
        DISABLE_HINTS.with(|c| c.set(prev));
    }
}

pub fn with_disable_hints(value: bool) -> DisableHintsGuard {
    let prev = DISABLE_HINTS.with(|c| c.replace(value));
    DisableHintsGuard { prev }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_false() {
        assert!(!disable_hints());
    }

    #[test]
    fn guard_rebinds_and_restores() {
        assert!(!disable_hints());
        {
            let _g = with_disable_hints(true);
            assert!(disable_hints());
        }
        assert!(!disable_hints());
    }

    #[test]
    fn nested_guards_stack() {
        let _outer = with_disable_hints(true);
        assert!(disable_hints());
        {
            let _inner = with_disable_hints(false);
            assert!(!disable_hints());
        }
        assert!(disable_hints());
    }
}
