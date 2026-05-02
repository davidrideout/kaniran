//! Port of `ichiran/characters:mora-length` (`characters.lisp:245-249`).
//!
//! Counts the number of "real" morae in a kana string, ignoring the
//! sokuon, all small kana modifiers (`ぁィゥェォ`, `ャュョ`), and the
//! long-vowel mark `ー`. Each excluded glyph either fuses with or
//! lengthens its neighbour rather than contributing a mora of its own.

const MODIFIERS: &str = "っッぁァぃィぅゥぇェぉォゃャゅュょョー";

pub fn mora_length(s: &str) -> usize {
    s.chars().filter(|c| !MODIFIERS.contains(*c)).count()
}
