//! Port of `ichiran/dict:top-array` (`dict.lisp:1140`).
//!
//! Bounded-size top-K accumulator used by `find-best-path` and the
//! per-segment-list scoring loop (`dict.lisp:1190-1230`). Carries a
//! fixed-length backing array of [`TopArrayItem`]s sorted highest
//! score first, plus a monotone insert counter.
//! `super::register_item::register_item` shifts new entries into
//! place; `super::get_array::get_array` returns the populated prefix.
//!
//! Slot shape (`(defclass top-array () ((array :reader top-array) (count :reader item-count :initform 0)))`):
//! - `array` — backing storage; per `slot_types.csv`. Lisp
//!   `initialize-instance :after` allocates with
//!   `(make-array limit :initial-element nil)`, so unset entries are
//!   `nil` until the registration loop overwrites them. Modeled as
//!   `Vec<Option<TopArrayItem>>` to preserve the "preallocated to
//!   `limit`, populated incrementally" shape.
//! - `count` — total number of registration calls; may exceed array
//!   length, in which case `get-array` returns the full array while
//!   later inserts continue to push out lowest scores.
//!
//! [`TopArray::new`] folds the Lisp `(make-instance 'top-array :limit n)`
//! plus the `initialize-instance :after` hook (`dict.lisp:1145-1146`)
//! into one constructor — every `make-instance 'top-array` callsite
//! (`dict.lisp:1192`, `:1197`) passes `:limit` explicitly, so the
//! Lisp `&key (limit 5)` default doesn't surface here; it lives on
//! `find-best-path` where the keyword is exposed.
//!
//! The Lisp `:reader item-count` and `:reader top-array` slot
//! readers collapse to direct public-field access on the struct,
//! matching the codebase precedent for ported classes (`counter-text`,
//! `word-info`, etc.).

use super::top_array_item_struct::TopArrayItem;

#[derive(Debug, Clone)]
pub struct TopArray {
    pub array: Vec<Option<TopArrayItem>>,
    pub count: usize,
}

impl TopArray {
    pub fn new(limit: usize) -> Self {
        Self {
            array: vec![None; limit],
            count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_preallocates_limit_with_nones() {
        let ta = TopArray::new(5);
        assert_eq!(ta.array.len(), 5);
        assert!(ta.array.iter().all(|x| x.is_none()));
        assert_eq!(ta.count, 0);
    }
}
