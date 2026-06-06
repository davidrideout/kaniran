//! Port of `ichiran/dict:get-pos` (`dict-load.lisp:253`, `csv-hash *pos-by-index*` accessor).
//!
//! Look up the part-of-speech tag for a numeric id in `*pos-by-index*`,
//! returning `None` on a miss.

use super::_star_pos_by_index_star_::pos_by_index;

pub fn get_pos(key: i32) -> Option<&'static str> {
    pos_by_index().get(&key).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103, after `(load-pos-by-index)`): `(get-pos id)` for
    /// present ids and a miss (`(get-pos 99999)` → nil).
    #[test]
    fn get_pos_lookups() {
        let cases: &[(i32, Option<&str>)] = &[
            (1, Some("adj-i")),
            (28, Some("v1")),
            (98, Some("unc")),
            (99999, None),
        ];
        for (key, expected) in cases {
            assert_eq!(get_pos(*key), *expected, "key={key}");
        }
    }
}
