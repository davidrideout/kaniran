//! Port of `ichiran/dict:top-array` (`dict.lisp:1140`).
//!
//! Bounded-size top-K accumulator used by `find-best-path`: a
//! fixed-length backing array of [`TopArrayItem`]s sorted highest
//! score first, plus a monotone insert counter. `count` may exceed the
//! array length while later inserts keep pushing out lowest scores.

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
