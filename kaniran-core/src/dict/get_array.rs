//! Port of `ichiran/dict:get-array` (`dict.lisp:1160`).
//!
//! Returns the populated prefix of a [`TopArray`]'s backing storage:
//! the whole array when `count` reaches its length, else the first
//! `count` slots.

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
