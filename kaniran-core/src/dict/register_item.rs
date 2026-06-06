//! Port of `ichiran/dict:register-item` (`dict.lisp:1148-1158`).
//!
//! Inserts `(score, payload)` into [`TopArray`]'s backing array,
//! maintaining the highest-score-first order by shifting lower-scored
//! entries down. `count` increments every call even when the array is
//! at capacity (the lowest-scored entry is dropped). Equal scores: the
//! new item lands BELOW the existing one.

use super::top_array_class::TopArray;
use super::top_array_item_struct::{PathElement, TopArrayItem};

pub fn register_item(obj: &mut TopArray, score: i32, payload: Vec<PathElement>) {
    // dict.lisp:1151 (let ((item (make-top-array-item :score score :payload payload)) (len ...)))
    let mut item: Option<TopArrayItem> = Some(TopArrayItem { score, payload });
    let len = obj.array.len();
    // dict.lisp:1153 (loop for idx from (min count len) downto 0 ...)
    let start = obj.count.min(len);
    let mut idx = start;
    loop {
        // dict.lisp:1154 (for prev-item = (when (> idx 0) (aref array (1- idx))))
        // The slot value is itself an Option (initialize-instance fills with nil);
        // both "idx == 0" and "slot is nil" collapse to None here.
        let prev_score = if idx > 0 {
            obj.array[idx - 1].as_ref().map(|prev| prev.score)
        } else {
            None
        };
        // dict.lisp:1155 (for done = (or (not prev-item) (>= (tai-score prev-item) score)))
        let done = match prev_score {
            None => true,
            Some(prev) => prev >= score,
        };
        // dict.lisp:1156 (when (< idx len) do (setf (aref array idx) (if done item prev-item)))
        if idx < len {
            obj.array[idx] = if done {
                item.take()
            } else {
                obj.array[idx - 1].take()
            };
        }
        // dict.lisp:1157 (until done)
        if done {
            break;
        }
        idx -= 1;
    }
    // dict.lisp:1158 (incf count)
    obj.count += 1;
}

#[cfg(test)]
mod tests {
    use super::super::segment_list_struct::SegmentList;
    use super::*;

    fn payload(tag: i32) -> Vec<PathElement> {
        // Use a sentinel SegmentList with `matches` carrying the tag —
        // tests only need a discriminable payload, not real data.
        vec![PathElement::SegmentList(SegmentList {
            segments: vec![],
            start: 0,
            end: 0,
            top: None,
            matches: tag as usize,
        })]
    }

    fn tag_of(item: &TopArrayItem) -> usize {
        match &item.payload[0] {
            PathElement::SegmentList(sl) => sl.matches,
            _ => panic!("unexpected payload variant"),
        }
    }

    fn scores_and_tags(obj: &TopArray) -> Vec<(i32, usize)> {
        obj.array
            .iter()
            .filter_map(|slot| slot.as_ref().map(|item| (item.score, tag_of(item))))
            .collect()
    }

    // REPL probes (`/tmp/probe_register_item.lisp` on .103, 2026-05-19).
    // Tags below correspond to the Lisp keyword payloads :A=1, :B=2,
    // :C=3, :Z=99 — only the structural relationships matter.

    #[test]
    fn a_empty_register_lands_at_index_0() {
        // REPL A: empty limit=3, register score=10 payload=(A).
        // count=1, [0]=(10,A), [1..]=NIL.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 10, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(scores_and_tags(&ta), vec![(10, 1)]);
        assert!(ta.array[1].is_none());
        assert!(ta.array[2].is_none());
    }

    #[test]
    fn b_higher_score_shifts_existing_down() {
        // REPL B: register 10/A then 20/B with limit=3.
        // count=2, [0]=(20,B), [1]=(10,A).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 2), (10, 1)]);
    }

    #[test]
    fn c_lower_score_appends_after() {
        // REPL C: register 20/A then 10/B with limit=3.
        // count=2, [0]=(20,A), [1]=(10,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 20, payload(1));
        register_item(&mut ta, 10, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 1), (10, 2)]);
    }

    #[test]
    fn d_equal_score_new_lands_below_existing() {
        // REPL D: register 20/A then 20/B — the `>=` test stops descent
        // at the equal slot, new item lands one index lower.
        // count=2, [0]=(20,A), [1]=(20,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 20, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 1), (20, 2)]);
    }

    #[test]
    fn e_middle_insert_shifts_lower_down() {
        // REPL E: limit=5, register 30/C, 10/A, 20/B.
        // count=3, [0]=(30,C), [1]=(20,B), [2]=(10,A), [3]=NIL, [4]=NIL.
        let mut ta = TopArray::new(5);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 3);
        assert_eq!(scores_and_tags(&ta), vec![(30, 3), (20, 2), (10, 1)]);
        assert!(ta.array[3].is_none());
        assert!(ta.array[4].is_none());
    }

    #[test]
    fn f_overflow_at_bottom_is_dropped_silently() {
        // REPL F: limit=3, register 30/C, 20/B, 10/A, 5/Z.
        // 5/Z's idx starts at min(3,3)=3 which fails the `(< idx len)`
        // guard; done is true (prev>=5) → break with no write.
        // count=4, [0..2]=(30,C),(20,B),(10,A); Z dropped.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 20, payload(2));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 5, payload(99));
        assert_eq!(ta.count, 4);
        assert_eq!(scores_and_tags(&ta), vec![(30, 3), (20, 2), (10, 1)]);
    }

    #[test]
    fn g_full_array_highest_replaces_top_evicts_lowest() {
        // REPL G: limit=3, register 30/C, 20/B, 10/A, then 99/Z.
        // 99/Z walks down through all three; the lowest (10/A) falls off
        // the bottom because idx=3 skip-writes (no out-of-bounds).
        // count=4, [0]=(99,Z), [1]=(30,C), [2]=(20,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 20, payload(2));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 99, payload(99));
        assert_eq!(ta.count, 4);
        assert_eq!(scores_and_tags(&ta), vec![(99, 99), (30, 3), (20, 2)]);
    }

    #[test]
    fn h_limit_one() {
        // REPL H: limit=1. After register 5/A: count=1, [0]=(5,A).
        // After register 10/B: B shifts A out → count=2, [0]=(10,B).
        // After register 3/C: 3 < 10, idx=1 skip-writes, done → count=3,
        // [0] still (10,B).
        let mut ta = TopArray::new(1);
        register_item(&mut ta, 5, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(scores_and_tags(&ta), vec![(5, 1)]);
        register_item(&mut ta, 10, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(10, 2)]);
        register_item(&mut ta, 3, payload(3));
        assert_eq!(ta.count, 3);
        assert_eq!(scores_and_tags(&ta), vec![(10, 2)]);
    }

    #[test]
    fn i_limit_zero_increments_count_only() {
        // REPL I: limit=0. No slot can be written; count still increments.
        // count=1, array empty.
        let mut ta = TopArray::new(0);
        register_item(&mut ta, 5, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(ta.array.len(), 0);
    }

    #[test]
    fn j_empty_payload() {
        // REPL J: find-best-path's initial call `(register-item top
        // (gap-penalty 0 str-length) nil)` — empty payload is the
        // structural zero element.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, -1000, vec![]);
        assert_eq!(ta.count, 1);
        let item = ta.array[0].as_ref().unwrap();
        assert_eq!(item.score, -1000);
        assert!(item.payload.is_empty());
    }
}
