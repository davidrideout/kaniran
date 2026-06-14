use super::*;

// --- add_deha_ja_readings ---
/// Rewrites a leading では to じゃ, including the exactly-two-char では.
#[test]
fn rewrite_deha_to_ja_cases() {
    let cases: &[(&str, &str)] = &[
        ("ではない", "じゃない"),
        ("ではなかった", "じゃなかった"),
        ("ではありませんでした", "じゃありませんでした"),
        ("ではないで", "じゃないで"),
        ("ではなくて", "じゃなくて"),
        ("ではなかったら", "じゃなかったら"),
        ("ではありませんでしたら", "じゃありませんでしたら"),
        ("ではありません", "じゃありません"),
        ("ではないです", "じゃないです"),
        ("では", "じゃ"),
    ];
    for (input, expected) in cases {
        assert_eq!(&rewrite_deha_to_ja(input), expected, "input={input}");
    }
}

// --- get_all_readings ---
/// All readings for an entry, plus a missing seq. The query has no
/// ORDER BY, so the returned order is unspecified and assertions
/// compare sorted sets.
#[tokio::test]
#[ignore = "DB test; requires KANIRAN_TEST_DATABASE_URL"]
async fn get_all_readings_corpus() {
    let ctx = kaniran_core::conn::kani_context::KaniranContext::from_env().expect("from_env");
    let cases: &[(i32, Vec<&str>)] = &[
        // seq 1582920 — この (with kanji writings 此の / 斯の and kana この / こん)
        (1582920, vec!["この", "こん", "斯の", "此の"]),
        // seq 1409080 — 駄弁 (with kanji writings 駄弁 / 駄辯 and kana だべん)
        (1409080, vec!["だべん", "駄弁", "駄辯"]),
        // seq 9000000 — no such entry
        (9000000, vec![]),
    ];
    for (seq, expected) in cases {
        let mut got = get_all_readings(&ctx, *seq).await.expect("query");
        got.sort();
        let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        want.sort();
        assert_eq!(got, want, "seq={seq}");
    }
}
