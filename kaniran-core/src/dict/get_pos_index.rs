//! Port of `ichiran/dict:get-pos-index` (`dict-load.lisp:249`, `csv-hash *pos-index*` accessor).
//!
//! Look up the numeric part-of-speech id for a tag string in
//! `*pos-index*`, returning `None` on a miss.

use super::_star_pos_index_star_::pos_index;

pub fn get_pos_index(key: &str) -> Option<i32> {
    // dict-load.lisp:251 — (val (car val)), val = (cons id description)
    pos_index().get(key).map(|val| val.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103, `ichiran/dict::get-pos-index`), 2026-05-24. Spot-checks
    /// across the kwpos.csv tags plus two misses (`nil` on absent key /
    /// empty string).
    #[test]
    fn get_pos_index_lookups() {
        let cases: &[(&str, Option<i32>)] = &[
            ("adj-i", Some(1)),
            ("adj-ix", Some(7)),
            ("v5aru", Some(30)),
            ("v1", Some(28)),
            ("v1-s", Some(29)),
            ("v5u", Some(41)),
            ("vs-s", Some(47)),
            ("v5r", Some(37)),
            ("n", Some(17)),
            ("nonexistent-pos", None),
            ("", None),
        ];
        for (key, expected) in cases {
            assert_eq!(get_pos_index(key), *expected, "key={key:?}");
        }
    }
}
