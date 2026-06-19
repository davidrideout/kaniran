use super::*;

// --- kanji_word_stats ---
// Result order is not part of the contract, so each assertion sorts both
// sides by key before comparing.

fn ctx() -> std::sync::Arc<KaniranContext> {
    crate::test_support::shared_ctx()
}

fn entry(rtext: &str, rtype: &str, count: i32) -> ((String, String), i32) {
    ((rtext.to_string(), rtype.to_string()), count)
}

#[test]
fn kanji_word_stats_fixtures() {
    let ctx = ctx();
    let cases: &[(&str, Vec<((String, String), i32)>, i32, usize)] = &[
        (
            "山",
            vec![entry("さん", "ja_on", 53), entry("やま", "ja_kun", 49)],
            2,
            104,
        ),
        (
            "水",
            vec![entry("すい", "ja_on", 136), entry("みず", "ja_kun", 48)],
            1,
            185,
        ),
        (
            "火",
            vec![
                entry("び", "ja_kun", 10),
                entry("ひ", "ja_kun", 17),
                entry("か", "ja_on", 45),
            ],
            3,
            75,
        ),
        (
            "心",
            vec![
                entry("こころ", "ja_kun", 25),
                entry("ごころ", "ja_kun", 8),
                entry("しん", "ja_on", 101),
            ],
            6,
            140,
        ),
        (
            "学",
            vec![entry("まな", "ja_kun", 2), entry("がく", "ja_on", 214)],
            0,
            216,
        ),
        (
            "国",
            vec![entry("くに", "ja_kun", 10), entry("こく", "ja_on", 195)],
            1,
            206,
        ),
        ("電", vec![entry("でん", "ja_on", 90)], 0, 90),
        (
            "鯨",
            vec![entry("げい", "ja_on", 2), entry("くじら", "ja_kun", 1)],
            0,
            3,
        ),
    ];
    for (kanji, expected_stats, expected_irregular, expected_total) in cases {
        let (stats, irregular, total) = kanji_word_stats(&ctx, kanji).unwrap();
        // The set of (reading, type) keys is the structural invariant and must
        // match exactly; the per-reading word counts and the total drift by a
        // word or two between backends (the archive's word set differs
        // slightly), so compare those with a small tolerance.
        let mut got_keys: Vec<(String, String)> =
            stats.iter().map(|(key, _)| key.clone()).collect();
        got_keys.sort();
        let mut want_keys: Vec<(String, String)> =
            expected_stats.iter().map(|(key, _)| key.clone()).collect();
        want_keys.sort();
        assert_eq!(got_keys, want_keys, "kanji={kanji:?} reading set");
        for (reading_key, want_count) in expected_stats {
            let got_count = stats
                .iter()
                .find(|(key, _)| key == reading_key)
                .map(|(_, count)| *count)
                .unwrap_or_else(|| panic!("kanji={kanji:?} missing reading {reading_key:?}"));
            crate::test_support::assert_approx_equal(got_count, *want_count, 2);
        }
        assert_eq!(irregular, *expected_irregular, "kanji={kanji:?} irregular");
        crate::test_support::assert_approx_equal(total as i32, *expected_total as i32, 2);
    }
}

// --- calculate_perc ---
#[test]
fn matches_repl_captures() {
    assert_eq!(calculate_perc(50, 100), "50.00%");
    assert_eq!(calculate_perc(1, 1000), "0.10%");
    assert_eq!(calculate_perc(0, 0), "--.--%");
    assert_eq!(calculate_perc(33, 100), "33.00%");
    assert_eq!(calculate_perc(1, 3), "33.33%");
    assert_eq!(calculate_perc(1, 7), "14.29%");
    assert_eq!(calculate_perc(5, 100), "5.00%");
    assert_eq!(calculate_perc(100, 100), "100.00%");
    assert_eq!(calculate_perc(3, 7), "42.86%");
}
