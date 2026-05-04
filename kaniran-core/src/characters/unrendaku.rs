//! Port of `ichiran/characters:unrendaku` (`characters.lisp:286-296`).
//!
//! Unvoice the first character of `txt`: maps `がガ → かカ`,
//! `ばバ/ぱパ → はハ`, `ゔヴ → うウ`, etc. via
//! [`super::_star_undakuten_hash_star_::undakuten_hash`]. The script
//! (hiragana vs. katakana) of the first glyph is preserved by aligning
//! by index inside the input class's `*kana-characters*` entry.
//!
//! Leaves `txt` unchanged when it's empty, when the first character has
//! no [`KanaClass`], or when the class has no unvoiced counterpart.
//!
//! The upstream signature is `(txt &key fresh)`. With `:fresh nil`
//! (default) it mutates `txt` in place; with `:fresh t` it copies first
//! and mutates the copy. The Rust port takes `&mut String` and always
//! mutates in place — equivalent to `:fresh nil`. Callers that need
//! `:fresh t` semantics clone before calling.

use super::_star_kana_characters_star_::KANA_CHARACTERS;
use super::_star_undakuten_hash_star_::undakuten_hash;
use super::get_char_class::get_char_class;
use super::kani_kana_class::KanaClass;

pub fn unrendaku(txt: &mut String) {
    let Some(first) = txt.chars().next() else {
        return;
    };
    let Some(cc) = get_char_class(first) else {
        return;
    };
    let Some(&unvoiced) = undakuten_hash().get(&cc) else {
        return;
    };
    let Some(new_char) = transpose(first, cc, unvoiced) else {
        return;
    };
    let first_byte_len = first.len_utf8();
    let mut buf = [0u8; 4];
    let new_str = new_char.encode_utf8(&mut buf);
    txt.replace_range(0..first_byte_len, new_str);
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
