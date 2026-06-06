//! Port of `ichiran/characters:consecutive-char-groups` (`characters.lisp:273-278`).
//!
//! Find every run of consecutive characters in `char_class` within
//! `s[start..end]` and return each as a `(start, end)` pair.
//!
//! Positions are *character* offsets, not byte offsets.

use super::_star_char_scanners_inner_star_::char_scanners_inner;
use super::char_class_type::CharClass;

pub fn consecutive_char_groups(
    char_class: CharClass,
    s: &str,
    start: usize,
    end: usize,
) -> Vec<(usize, usize)> {
    let re = char_scanners_inner()
        .get(&char_class)
        .expect("char_class is in *char-scanners-inner*");
    let start_byte = nth_char_byte(s, start);
    let end_byte = nth_char_byte(s, end);
    let slice = &s[start_byte..end_byte];
    re.find_iter(slice)
        .map(|m| m.expect("regex iteration"))
        .map(|m| {
            let s_char = start + slice[..m.start()].chars().count();
            let e_char = start + slice[..m.end()].chars().count();
            (s_char, e_char)
        })
        .collect()
}

fn nth_char_byte(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Positions are character offsets, not byte offsets.
    #[test]
    fn returns_character_offsets_not_byte_offsets() {
        let s = "あ12い34";
        // chars: 0='あ' 1='1' 2='2' 3='い' 4='3' 5='4'
        assert_eq!(
            consecutive_char_groups(CharClass::Number, s, 0, s.chars().count()),
            vec![(1, 3), (4, 6)],
        );
    }
}
