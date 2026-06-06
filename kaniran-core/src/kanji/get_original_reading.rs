//! Port of `ichiran/kanji:get-original-reading` (`kanji.lisp:308`).
//!
//! Recovers the underlying kun/on dictionary form from a reading
//! variant: strips dakuten/handakuten when `rendaku` is set, and
//! replaces the trailing character with the supplied `geminated` glyph
//! when present.

use crate::characters::unrendaku::unrendaku;

pub fn get_original_reading(
    rtext: &str,
    rendaku: bool,
    geminated: Option<&str>,
) -> String {
    let mut s = rtext.to_string();
    if rendaku {
        unrendaku(&mut s);
    }
    if let Some(g) = geminated {
        // kanji.lisp:311 ((setf (char rtext (1- (length rtext))) (char geminated 0)))
        let new_first = g.chars().next().expect("geminated is non-empty when present");
        let last_pos = s
            .char_indices()
            .last()
            .expect("rtext is non-empty when geminated is set")
            .0;
        let mut buf = [0u8; 4];
        let new_str = new_first.encode_utf8(&mut buf);
        s.replace_range(last_pos.., new_str);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_repl_captures() {
        // /tmp/probe-kanji.lisp on .103 — verified 2026-05-09.
        assert_eq!(get_original_reading("はる", false, None), "はる");
        assert_eq!(get_original_reading("ばる", true, None), "はる");
        assert_eq!(get_original_reading("はつ", false, Some("つ")), "はつ");
        assert_eq!(get_original_reading("ばっ", true, Some("つ")), "はつ");
    }
}
