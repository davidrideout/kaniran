//! Port of `ichiran/dict:load-pos-by-index` (`dict-load.lisp:253`, `csv-hash *pos-by-index*` expansion).
//!
//! Build the numeric id → part-of-speech tag map from the embedded
//! kwpos.csv (tab-separated, header skipped — mirrors `cl-csv:read-csv
//! :separator #\Tab :skip-first-p t`). The `description` / `ents`
//! columns are unused by this loader's value-form (`pos`).
//!
//! Diverges from upstream `(merge-pathnames *jmdict-data* "kwpos.csv")`:
//! kwpos.csv is vendored into the crate and embedded with `include_str!`.
//! Returns the built map; the upstream `setf` of `*pos-by-index*` lives
//! on the `OnceLock` in [`super::_star_pos_by_index_star_`].

use std::collections::HashMap;

const KWPOS_CSV: &str = include_str!("../../data/kwpos.csv");

pub fn load_pos_by_index() -> HashMap<i32, String> {
    let mut hash = HashMap::new();
    for row in KWPOS_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        // dict-load.lisp:254 — row-key-form (parse-integer pos-id), value-form pos
        let pos_id = cols
            .next()
            .expect("kwpos.csv row missing id column")
            .parse::<i32>()
            .expect("kwpos.csv id column not an integer");
        let pos = cols.next().expect("kwpos.csv row missing kw column");
        hash.insert(pos_id, pos.to_string());
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103, after `(load-pos-by-index)`): `(hash-table-count
    /// *pos-by-index*)` = 92 (93 kwpos.csv lines − header). Spot-checks
    /// pin the tab-split and the id→tag value.
    #[test]
    fn loads_kwpos_csv() {
        let map = load_pos_by_index();
        assert_eq!(map.len(), 92);

        let cases: &[(i32, &str)] = &[(1, "adj-i"), (28, "v1"), (98, "unc")];
        for (pos_id, pos) in cases {
            assert_eq!(map.get(pos_id).map(String::as_str), Some(*pos), "id={pos_id}");
        }

        // header row not parsed as data
        assert_eq!(map.get(&0), None);
    }
}
