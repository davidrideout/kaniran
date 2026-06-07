use super::*;

// --- _star_pos_by_index_star_ ---
/// The `OnceLock` builder wiring runs `load_pos_by_index` (92
/// entries per the .103 REPL).
#[test]
fn pos_by_index_builds_once() {
    assert_eq!(pos_by_index().len(), 92);
}

// --- _star_pos_index_star_ ---
/// The `OnceLock` builder wiring runs `load_pos_index` (92 entries
/// per the .103 REPL).
#[test]
fn pos_index_builds_once() {
    assert_eq!(pos_index().len(), 92);
}

// --- get_pos_index ---
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

// --- load_pos_index ---
/// REPL (.103, after `(load-pos-index)`): `(hash-table-count
/// *pos-index*)` = 92 (93 kwpos.csv lines − header). Spot-checks
/// pin the tab-split, the `(id . description)` value, and the
/// dropped header / `ents` column.
#[test]
fn load_pos_index_loads_kwpos_csv() {
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

// --- get_pos ---
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

// --- load_pos_by_index ---
/// REPL (.103, after `(load-pos-by-index)`): `(hash-table-count
/// *pos-by-index*)` = 92 (93 kwpos.csv lines − header). Spot-checks
/// pin the tab-split and the id→tag value.
#[test]
fn load_pos_by_index_loads_kwpos_csv() {
    let map = load_pos_by_index();
    assert_eq!(map.len(), 92);

    let cases: &[(i32, &str)] = &[(1, "adj-i"), (28, "v1"), (98, "unc")];
    for (pos_id, pos) in cases {
        assert_eq!(
            map.get(pos_id).map(String::as_str),
            Some(*pos),
            "id={pos_id}"
        );
    }

    // header row not parsed as data
    assert_eq!(map.get(&0), None);
}
