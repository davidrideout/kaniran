//! Port of `ichiran/kanji:get-original-reading` (`kanji.lisp:308`).
//!
//! Recovers the underlying kun/on dictionary form from a reading
//! variant. Strips dakuten/handakuten via [`super::super::characters::unrendaku`]
//! when `rendaku` is set, and replaces the trailing character with
//! the supplied `geminated` glyph (typically the original mora before
//! gemination produced `っ`) when present.
//!
//! Diverges from the upstream lambda list `(rtext &optional rendaku
//! geminated)` by:
//!
//! - taking `rendaku` as a plain `bool` per CONVENTIONS §4.4. The
//!   keyword is binary upstream — `:rendaku` (truthy) or `nil` —
//!   matching how [`super::get_reading_alternatives::ReadingTag`]
//!   records the tag at the producing call site.
//! - taking `geminated` as `Option<&str>`. Upstream accepts a string
//!   whose first character is grafted onto the result; `None` mirrors
//!   the absent / `nil` case.
//! - returning a fresh `String`. Upstream's `:fresh t` semantics for
//!   [`super::super::characters::unrendaku`] are reproduced by cloning
//!   `rtext` upfront and mutating the clone.

use crate::characters::voicing::unrendaku;

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
