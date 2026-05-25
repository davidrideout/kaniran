//! Port of `ichiran:load-romaji-kana` (`deromanize.lisp:7`, `csv-hash *romaji-kana*` expansion).
//!
//! Build the romaji-prefix → [`RmapItem`] map: parse the vendored
//! romaji-map.csv (tab-separated, no header — mirrors `cl-csv:read-csv
//! :separator #\Tab :skip-first-p nil`). Each row is `text<TAB>kana`
//! with an optional third `next` column (present only on
//! doubled-consonant gemination rows); absent → `None`. The `text`
//! column is the key, so the duplicate `fu` row collapses to one
//! entry (292 keys from 293 rows).
//!
//! Diverges from upstream `(asdf:system-relative-pathname :ichiran
//! "data/romaji-map.csv")`: the CSV is vendored into the crate and
//! embedded with `include_str!`. Returns the built map; the upstream
//! `setf` of `*romaji-kana*` lives on the `OnceLock` in
//! [`super::_star_romaji_kana_star_`].

use std::collections::HashMap;

use super::rmap_item_struct::RmapItem;

const ROMAJI_MAP_CSV: &str = include_str!("../../data/romaji-map.csv");

pub fn load_romaji_kana() -> HashMap<String, RmapItem> {
    let mut hash = HashMap::new();
    for row in ROMAJI_MAP_CSV.lines() {
        let mut cols = row.split('\t');
        let text = cols.next().expect("romaji-map.csv row missing text column");
        let kana = cols.next().expect("romaji-map.csv row missing kana column");
        let next = cols.next();
        hash.insert(
            text.to_string(),
            RmapItem {
                text: text.to_string(),
                kana: kana.to_string(),
                next: next.map(str::to_string),
            },
        );
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL (.103, after `(load-romaji-kana)`): 292 keys (293 rows,
    /// `fu` duplicated). Spot-checks pin the tab-split parse, the
    /// 2-column `next`→`None`, and the 3-column gemination `next`.
    #[test]
    fn loads_romaji_map_csv() {
        let map = load_romaji_kana();
        assert_eq!(map.len(), 292);

        let a = &map["a"];
        assert_eq!((a.text.as_str(), a.kana.as_str(), a.next.as_deref()), ("a", "あ", None));
        let n = &map["n"];
        assert_eq!((n.text.as_str(), n.kana.as_str(), n.next.as_deref()), ("n", "ん", None));
        let di = &map["d'i"];
        assert_eq!((di.text.as_str(), di.kana.as_str(), di.next.as_deref()), ("d'i", "でぃ", None));
        let bb = &map["bb"];
        assert_eq!((bb.text.as_str(), bb.kana.as_str(), bb.next.as_deref()), ("bb", "っ", Some("b")));
        let mm = &map["mm"];
        assert_eq!((mm.text.as_str(), mm.kana.as_str(), mm.next.as_deref()), ("mm", "ん", Some("m")));
    }
}
