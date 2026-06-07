use super::*;

// --- _star_conj_description_star_ ---
/// The `OnceLock` builder wiring runs `load_conj_description`
/// (18 entries per the .103 REPL).
#[test]
fn conj_description_builds_once() {
    assert_eq!(conj_description().len(), 18);
}

// --- _star_conj_rules_star_ ---
/// REPL probe (after `(load-conj-rules)`): `(hash-table-count
/// *conj-rules*)` = 24 (one entry per pos that appears in
/// conjo.csv, plus errata insertions for adj-i / adj-ix / v5aru /
/// vs-s already-present pos ids).
#[test]
fn conj_rules_builds_once() {
    assert_eq!(conj_rules().len(), 24);
}

// --- get_conj_description ---
/// REPL fixtures (.103, ichiran/dict::get-conj-description), 2026-05-24.
/// Present ids resolve to their description; an absent id returns
/// `None` (upstream nil).
#[test]
fn get_conj_description_fixtures() {
    let cases: &[(i32, Option<&str>)] = &[
        (1, Some("Non-past")),
        (2, Some("Past (~ta)")),
        (11, Some("Conditional (~tara)")),
        (50, Some("Adverbial")),
        (999, None),
    ];
    for (key, expected) in cases {
        assert_eq!(get_conj_description(*key), *expected, "key={key}");
    }
}

// --- load_conj_description ---
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

// --- get_conj_rules ---
/// REPL probe (`(ichiran/dict::get-conj-rules pos-id)`), 2026-05-31.
/// The cases pin both the post-errata row count and the first/last
/// rules in CSV / accessor order — accessor reverses the stored
/// cons-prepend list so the last element is the errata-injected
/// `(conj=52)` v5* negative-stem when the pos starts with `v5`.
#[test]
fn get_conj_rules_lookups() {
    // pos 1 (adj-i): 23 rules; first conj=1 okuri="い", last conj=54 okuri="き"
    let r1 = get_conj_rules(1);
    assert_eq!(r1.len(), 23);
    assert_eq!(
        r1.first().map(|r| (r.conj, r.okuri.as_str())),
        Some((1, "い"))
    );
    assert_eq!(
        r1.last().map(|r| (r.conj, r.okuri.as_str())),
        Some((54, "き"))
    );

    // pos 41 (v5u): 61 rules; first conj=1 okuri="う"; last 2 rules pinned
    // (conj=13 okuri="い" from CSV, then errata-appended conj=52 okuri="わ").
    let r41 = get_conj_rules(41);
    assert_eq!(r41.len(), 61);
    assert_eq!(
        r41.first().map(|r| (r.conj, r.okuri.as_str())),
        Some((1, "う"))
    );
    let tail2: Vec<_> = r41[r41.len() - 2..]
        .iter()
        .map(|r| (r.conj, r.okuri.as_str(), r.neg, r.fml))
        .collect();
    assert_eq!(
        tail2,
        vec![(13, "い", false, false), (52, "わ", true, false)]
    );

    // pos 28 (v1): 60 rules per the REPL probe.
    assert_eq!(get_conj_rules(28).len(), 60);

    // pos 30 (v5aru): 62 rules per the REPL probe.
    assert_eq!(get_conj_rules(30).len(), 62);

    // pos 47 (vs-s): 53 rules after potential-form (conj=5) removal.
    let r47 = get_conj_rules(47);
    assert_eq!(r47.len(), 53);
    // dict-errata.lisp:1287 removes conj=5 from vs-s entirely.
    assert!(r47.iter().all(|r| r.conj != 5));

    // missing key → empty (upstream returns nil; (reverse nil) = nil)
    assert!(get_conj_rules(9999).is_empty());
}
