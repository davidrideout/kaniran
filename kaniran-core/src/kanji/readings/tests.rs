use super::*;

// --- get_original_reading ---
#[test]
fn matches_repl_captures() {
    assert_eq!(get_original_reading("はる", false, None), "はる");
    assert_eq!(get_original_reading("ばる", true, None), "はる");
    assert_eq!(get_original_reading("はつ", false, Some("つ")), "はつ");
    assert_eq!(get_original_reading("ばっ", true, Some("つ")), "はつ");
}
