use super::*;

// --- _star_romaji_kana_star_ ---
/// The `OnceLock` builder wiring runs `load_romaji_kana` (292
/// entries per the .103 REPL).
#[test]
fn romaji_kana_builds_once() {
    assert_eq!(romaji_kana().len(), 292);
}

// --- get_romaji_kana ---
#[test]
fn get_romaji_kana_fixtures() {
    // REPL fixtures (.103, ichiran::get-romaji-kana), 2026-05-26.
    // (key, Some(text, kana, next) | None). Covers a plain rule, a
    // doubled-consonant rule (`next` present), a missing key, and
    // the empty key.
    let cases: &[(&str, Option<(&str, &str, Option<&str>)>)] = &[
        ("a", Some(("a", "あ", None))),
        ("ka", Some(("ka", "か", None))),
        ("shi", Some(("shi", "し", None))),
        ("n", Some(("n", "ん", None))),
        ("bb", Some(("bb", "っ", Some("b")))),
        ("kk", Some(("kk", "っ", Some("k")))),
        ("pp", Some(("pp", "っ", Some("p")))),
        ("xyz", None),
        ("", None),
    ];
    for (key, expected) in cases {
        let got = get_romaji_kana(key)
            .map(|rmi| (rmi.text.as_str(), rmi.kana.as_str(), rmi.next.as_deref()));
        assert_eq!(got, *expected, "key={key:?}");
    }
}

// --- load_romaji_kana ---
/// REPL (.103, after `(load-romaji-kana)`): 292 keys (293 rows,
/// `fu` duplicated). Spot-checks pin the tab-split parse, the
/// 2-column `next`→`None`, and the 3-column gemination `next`.
#[test]
fn loads_romaji_map_csv() {
    let map = load_romaji_kana();
    assert_eq!(map.len(), 292);

    let a = &map["a"];
    assert_eq!(
        (a.text.as_str(), a.kana.as_str(), a.next.as_deref()),
        ("a", "あ", None)
    );
    let n = &map["n"];
    assert_eq!(
        (n.text.as_str(), n.kana.as_str(), n.next.as_deref()),
        ("n", "ん", None)
    );
    let di = &map["d'i"];
    assert_eq!(
        (di.text.as_str(), di.kana.as_str(), di.next.as_deref()),
        ("d'i", "でぃ", None)
    );
    let bb = &map["bb"];
    assert_eq!(
        (bb.text.as_str(), bb.kana.as_str(), bb.next.as_deref()),
        ("bb", "っ", Some("b"))
    );
    let mm = &map["mm"];
    assert_eq!(
        (mm.text.as_str(), mm.kana.as_str(), mm.next.as_deref()),
        ("mm", "ん", Some("m"))
    );
}

// --- _star_romaji_kana_next_star_ ---
/// REPL (.103): `(hash-table-count *romaji-kana-next*)` = 60.
/// Membership spot-checks pin the proper-prefix contract: short
/// prefixes of longer keys are present, full keys are not.
#[test]
fn romaji_kana_next_builds_once() {
    let next = romaji_kana_next();
    assert_eq!(next.len(), 60);
    for present in ["k", "ky", "c", "ch", "b", "s", "sh"] {
        assert!(next.contains(present), "expected {present:?} present");
    }
    for absent in ["ka", "a", "zz"] {
        assert!(!next.contains(absent), "expected {absent:?} absent");
    }
}
