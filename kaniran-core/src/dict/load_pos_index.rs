//! Port of `ichiran/dict:load-pos-index` (`dict-load.lisp:249`, `csv-hash *pos-index*` expansion).
//!
//! Build the part-of-speech → (id, description) map from the embedded
//! kwpos.csv (tab-separated, header skipped — mirrors `cl-csv:read-csv
//! :separator #\Tab :skip-first-p t`). The trailing `ents` column is
//! dropped, as in the upstream `(pos-id pos description)` row binding.
//!
//! Diverges from upstream `(merge-pathnames *jmdict-data* "kwpos.csv")`:
//! kwpos.csv is vendored into the crate and embedded with `include_str!`.
//! Returns the built map; the upstream `setf` of `*pos-index*` lives on
//! the `OnceLock` in [`super::_star_pos_index_star_`].

use std::collections::HashMap;

const KWPOS_CSV: &str = include_str!("../../data/kwpos.csv");

pub fn load_pos_index() -> HashMap<String, (i32, String)> {
    let mut hash = HashMap::new();
    for row in KWPOS_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        let pos_id = cols
            .next()
            .expect("kwpos.csv row missing id column")
            .parse::<i32>()
            .expect("kwpos.csv id column not an integer");
        let pos = cols.next().expect("kwpos.csv row missing kw column");
        let description = cols.next().expect("kwpos.csv row missing descr column");
        // dict-load.lisp:250 — value-form (cons (parse-integer pos-id) description)
        hash.insert(pos.to_string(), (pos_id, description.to_string()));
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103, after `(load-pos-index)`): `(hash-table-count
    /// *pos-index*)` = 92 (93 kwpos.csv lines − header). Spot-checks
    /// pin the tab-split, the `(id . description)` value, and the
    /// dropped header / `ents` column.
    #[test]
    fn loads_kwpos_csv() {
        let map = load_pos_index();
        assert_eq!(map.len(), 92);

        let cases: &[(&str, (i32, &str))] = &[
            ("adj-i", (1, "adjective (keiyoushi)")),
            ("v5u", (41, "Godan verb with 'u' ending")),
            ("unc", (98, "unclassified")),
        ];
        for (pos, (pos_id, description)) in cases {
            let value = map.get(*pos).expect("pos present");
            assert_eq!(value.0, *pos_id, "pos={pos}");
            assert_eq!(value.1.as_str(), *description, "pos={pos}");
        }

        // header row not parsed as data
        assert_eq!(map.get("kw"), None);
    }
}
