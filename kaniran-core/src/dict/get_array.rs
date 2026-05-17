//! Port of `ichiran/dict:get-array` (`dict.lisp:1160`).
//!
//! Returns the populated prefix of a [`TopArray`]'s backing storage.
//! Upstream:
//!
//! ```lisp
//! (defgeneric get-array (collection)
//!   (:method ((obj top-array))
//!     (with-slots (array count) obj
//!       (if (>= count (length array)) array (subseq array 0 count)))))
//! ```
//!
//! The Lisp generic has a single method on `top-array`; with no
//! polymorphic dispatch the Rust port is a free function taking
//! `&TopArray` per CONVENTIONS §4.7 precedent (see `score_base`).
//!
//! When `count` is below the array length, only the first `count`
//! slots are populated (`register-item` writes incrementally); the
//! remaining slots stay at the `initialize-instance :after` `nil`
//! fill. When `count` is at or above the array length, every slot is
//! populated and the eviction loop in `register-item` keeps the
//! highest scores. Either way callers (`find-best-path` at
//! `dict.lisp:1215`, `:1232`) iterate `for tai across` the result and
//! see only the populated entries.
//!
//! Divergences from Lisp:
//! - Returns a borrowed slice `&[Option<TopArrayItem>]` rather than
//!   either the underlying array or a freshly-allocated `subseq`. The
//!   only consumers are `for tai across` iterations; the iteration
//!   semantics are identical and the borrow avoids an unnecessary
//!   allocation. Per CONVENTIONS §4.9.

use super::top_array_class::TopArray;
use super::top_array_item_struct::TopArrayItem;

pub fn get_array(obj: &TopArray) -> &[Option<TopArrayItem>] {
    if obj.count >= obj.array.len() {
        &obj.array
    } else {
        &obj.array[0..obj.count]
    }
}

#[cfg(test)]
mod tests {
    use super::super::segment_list_struct::SegmentList;
    use super::super::top_array_item_struct::{PathElement, TopArrayItem};
    use super::*;

    fn dummy_payload(score: i32) -> TopArrayItem {
        TopArrayItem {
            score,
            payload: vec![PathElement::SegmentList(SegmentList {
                segments: vec![],
                start: 0,
                end: 0,
                top: None,
                matches: 0,
            })],
        }
    }

    #[test]
    fn empty_top_array_returns_empty_slice() {
        // REPL: empty len=0
        let ta = TopArray::new(3);
        assert_eq!(get_array(&ta).len(), 0);
    }

    #[test]
    fn partial_returns_first_count_slots() {
        // REPL: after 1 register, len=1, first score=50
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(50));
        ta.count = 1;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_ref().unwrap().score, 50);
    }

    #[test]
    fn count_equal_to_len_returns_full() {
        // REPL: after 3, len=3 scores=(100 50 10)
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(100));
        ta.array[1] = Some(dummy_payload(50));
        ta.array[2] = Some(dummy_payload(10));
        ta.count = 3;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr.iter()
                .map(|x| x.as_ref().unwrap().score)
                .collect::<Vec<_>>(),
            vec![100, 50, 10]
        );
    }

    #[test]
    fn count_exceeding_len_returns_full() {
        // REPL: after 4 (overflow), len=3 scores=(999 100 50)
        // count exceeds array.len() but we still return the whole array.
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(999));
        ta.array[1] = Some(dummy_payload(100));
        ta.array[2] = Some(dummy_payload(50));
        ta.count = 4;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr.iter()
                .map(|x| x.as_ref().unwrap().score)
                .collect::<Vec<_>>(),
            vec![999, 100, 50]
        );
    }
}
