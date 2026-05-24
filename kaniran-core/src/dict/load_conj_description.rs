//! Port of `ichiran/dict:load-conj-description` (`dict-load.lisp:257`, `csv-hash *conj-description*` expansion).
//!
//! Build the conj-id → description map: parse the embedded conj.csv
//! (tab-separated, header skipped — mirrors `cl-csv:read-csv
//! :separator #\Tab :skip-first-p t`), then apply
//! [`errata_conj_description_hook`].
//!
//! Diverges from upstream `(merge-pathnames *jmdict-data* "conj.csv")`:
//! conj.csv is an external jmdictdb file, vendored into the crate and
//! embedded with `include_str!`, so kaniran needs no jmdictdb checkout.
//! Returns the built map; the upstream `setf` of `*conj-description*`
//! lives on the `OnceLock` in [`super::_star_conj_description_star_`].

use std::collections::HashMap;

use super::errata_conj_description_hook::errata_conj_description_hook;

const CONJ_CSV: &str = include_str!("../../data/conj.csv");

pub fn load_conj_description() -> HashMap<i32, String> {
    let mut hash = HashMap::new();
    for row in CONJ_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        let conj_id = cols
            .next()
            .expect("conj.csv row missing id column")
            .parse::<i32>()
            .expect("conj.csv id column not an integer");
        let description = cols.next().expect("conj.csv row missing name column");
        hash.insert(conj_id, description.to_string());
    }
    errata_conj_description_hook(&mut hash);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103, after `(load-conj-description)`): 18 entries — 13
    /// conj.csv rows + 5 from the errata hook. Spot-checks pin the
    /// tab-split parse (a `(~tara)` value), the CSV bounds, and the
    /// errata boundary keys (50, 54).
    #[test]
    fn loads_conj_csv_with_errata() {
        let map = load_conj_description();
        assert_eq!(map.len(), 18);

        let cases: &[(i32, &str)] = &[
            (1, "Non-past"),
            (11, "Conditional (~tara)"),
            (13, "Continuative (~i)"),
            (50, "Adverbial"),         // +conj-adverbial+ (errata)
            (54, "Old/literary form"), // +conj-adjective-literary+ (errata)
        ];
        for (conj_id, description) in cases {
            assert_eq!(
                map.get(conj_id).map(String::as_str),
                Some(*description),
                "conj_id={conj_id}"
            );
        }

        // header row not parsed; out-of-range / between-range keys absent
        assert_eq!(map.get(&0), None);
        assert_eq!(map.get(&14), None);
        assert_eq!(map.get(&999), None);
    }
}
