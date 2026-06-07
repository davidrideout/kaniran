use super::*;

// --- _star_pos_by_index_star_ ---
/// The pos-by-index table has 92 entries.
#[test]
fn pos_by_index_builds_once() {
    assert_eq!(pos_by_index().len(), 92);
}

// --- _star_pos_index_star_ ---
/// The pos-index table has 92 entries.
#[test]
fn pos_index_builds_once() {
    assert_eq!(pos_index().len(), 92);
}

// --- get_pos_index ---
/// Looks up the index for part-of-speech tags from kwpos.csv, plus two
/// misses (absent key and empty string).
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
/// Loads kwpos.csv into 92 entries (93 lines minus the header), each tag
/// mapping to its `(id, description)`.
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
/// Looks up the tag for present ids, plus a miss on an absent id.
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
/// Loads kwpos.csv into 92 entries (93 lines minus the header), each id
/// mapping to its tag.
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
