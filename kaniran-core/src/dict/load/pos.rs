use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `ichiran/dict:*pos-by-index*` (`dict-load.lisp:253`, `csv-hash *pos-by-index*`).
///
/// numeric id → part-of-speech tag, from the vendored kwpos.csv.
pub fn pos_by_index() -> &'static HashMap<i32, String> {
    static POS_BY_INDEX: OnceLock<HashMap<i32, String>> = OnceLock::new();
    POS_BY_INDEX.get_or_init(load_pos_by_index)
}

/// Port of `ichiran/dict:*pos-index*` (`dict-load.lisp:249`, `csv-hash *pos-index*`).
///
/// part-of-speech tag → (numeric id, English description), from the
/// vendored kwpos.csv.
pub fn pos_index() -> &'static HashMap<String, (i32, String)> {
    static POS_INDEX: OnceLock<HashMap<String, (i32, String)>> = OnceLock::new();
    POS_INDEX.get_or_init(load_pos_index)
}

/// Port of `ichiran/dict:get-pos-index` (`dict-load.lisp:249`, `csv-hash *pos-index*` accessor).
///
/// Look up the numeric part-of-speech id for a tag string in
/// `*pos-index*`, returning `None` on a miss.
pub fn get_pos_index(key: &str) -> Option<i32> {
    // dict-load.lisp:251 — (val (car val)), val = (cons id description)
    pos_index().get(key).map(|val| val.0)
}

/// Port of `ichiran/dict:load-pos-index` (`dict-load.lisp:249`, `csv-hash *pos-index*` expansion).
///
/// Build the part-of-speech → (id, description) map from kwpos.csv
/// (tab-separated, header skipped); the trailing `ents` column is dropped.
const LOAD_POS_INDEX_KWPOS_CSV: &str = include_str!("../../../data/kwpos.csv");

pub fn load_pos_index() -> HashMap<String, (i32, String)> {
    let mut hash = HashMap::new();
    for row in LOAD_POS_INDEX_KWPOS_CSV.lines().skip(1) {
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

/// Port of `ichiran/dict:get-pos` (`dict-load.lisp:253`, `csv-hash *pos-by-index*` accessor).
///
/// Look up the part-of-speech tag for a numeric id in `*pos-by-index*`,
/// returning `None` on a miss.
pub fn get_pos(key: i32) -> Option<&'static str> {
    pos_by_index().get(&key).map(String::as_str)
}

/// Port of `ichiran/dict:load-pos-by-index` (`dict-load.lisp:253`, `csv-hash *pos-by-index*` expansion).
///
/// Builds the numeric id → part-of-speech tag map by parsing the
/// tab-separated kwpos.csv (header skipped); only the id and tag
/// columns are used.
const LOAD_POS_BY_INDEX_KWPOS_CSV: &str = include_str!("../../../data/kwpos.csv");

pub fn load_pos_by_index() -> HashMap<i32, String> {
    let mut hash = HashMap::new();
    for row in LOAD_POS_BY_INDEX_KWPOS_CSV.lines().skip(1) {
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
mod tests;
