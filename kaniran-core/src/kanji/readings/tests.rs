use super::*;

// --- get_original_reading ---
#[test]
fn matches_repl_captures() {
    // /tmp/probe-kanji.lisp on .103 — verified 2026-05-09.
    assert_eq!(get_original_reading("はる", false, None), "はる");
    assert_eq!(get_original_reading("ばる", true, None), "はる");
    assert_eq!(get_original_reading("はつ", false, Some("つ")), "はつ");
    assert_eq!(get_original_reading("ばっ", true, Some("つ")), "はつ");
}
