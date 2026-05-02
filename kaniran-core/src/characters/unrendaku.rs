//! Port of `ichiran/characters:unrendaku` (`characters.lisp:286-296`).
//!
//! Unvoice the first character of `txt`: maps `がガ → かカ`,
//! `ばバ/ぱパ → はハ`, `ゔヴ → うウ`, etc. via
//! [`super::_star_undakuten_hash_star_::undakuten_hash`]. The script
//! (hiragana vs. katakana) of the first glyph is preserved by aligning
//! by index inside the input class's `*kana-characters*` entry.
//!
//! Returns `txt` unchanged when it's empty, when the first character
//! has no [`KanaClass`], or when the class has no unvoiced counterpart.
//! Per CONVENTIONS §4.6 the `:fresh` keyword is dropped — the Rust
//! port always allocates a fresh `String`.

use super::_star_kana_characters_star_::KANA_CHARACTERS;
use super::_star_undakuten_hash_star_::undakuten_hash;
use super::get_char_class::get_char_class;
use super::kani_kana_class::KanaClass;

pub fn unrendaku(txt: &str) -> String {
    let mut chars: Vec<char> = txt.chars().collect();
    let Some(&first) = chars.first() else {
        return String::new();
    };
    let Some(cc) = get_char_class(first) else {
        return txt.to_string();
    };
    let Some(&unvoiced) = undakuten_hash().get(&cc) else {
        return txt.to_string();
    };
    let Some(new_char) = transpose(first, cc, unvoiced) else {
        return txt.to_string();
    };
    chars[0] = new_char;
    chars.into_iter().collect()
}

/// Find `c`'s position inside `KANA_CHARACTERS[from]`, then return the
/// glyph at the same position in `KANA_CHARACTERS[to]`. Used by
/// [`unrendaku`] and [`super::rendaku::rendaku`] to preserve the
/// hiragana/katakana script of the input.
pub(super) fn transpose(c: char, from: KanaClass, to: KanaClass) -> Option<char> {
    let from_str = lookup_kana(from)?;
    let to_str = lookup_kana(to)?;
    let pos = from_str.chars().position(|x| x == c)?;
    to_str.chars().nth(pos)
}

fn lookup_kana(cc: KanaClass) -> Option<&'static str> {
    KANA_CHARACTERS
        .iter()
        .find_map(|(k, s)| if *k == cc { Some(*s) } else { None })
}
