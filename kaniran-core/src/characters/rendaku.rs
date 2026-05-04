//! Port of `ichiran/characters:rendaku` (`characters.lisp:298-309`).
//!
//! Voice the first character of `txt` — `かカ → がガ`, `はハ → ばバ`,
//! etc. With [`Voicing::Handakuten`] the H-row picks up `゜` instead of
//! `゛` (`はハ → ぱパ`); other rows have no handakuten form so the
//! input passes through unchanged. The hiragana/katakana script of the
//! first glyph is preserved.
//!
//! Leaves `txt` unchanged when it's empty, when the first character has
//! no [`KanaClass`], or when the class has no voiced counterpart in the
//! chosen hash.
//!
//! The upstream signature is `(txt &key fresh handakuten)`. With
//! `:fresh nil` (default) it mutates `txt` in place; with `:fresh t` it
//! copies first and mutates the copy. The Rust port takes `&mut String`
//! and always mutates in place — equivalent to `:fresh nil`. Callers
//! that need `:fresh t` semantics clone before calling. The
//! `:handakuten` boolean becomes a 2-variant [`Voicing`] enum
//! (CONVENTIONS §4.4).

use super::_star_dakuten_hash_star_::dakuten_hash;
use super::_star_handakuten_hash_star_::handakuten_hash;
use super::get_char_class::get_char_class;
use super::unrendaku::transpose;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voicing {
    Dakuten,
    Handakuten,
}

pub fn rendaku(txt: &mut String, voicing: Voicing) {
    let Some(first) = txt.chars().next() else {
        return;
    };
    let Some(cc) = get_char_class(first) else {
        return;
    };
    let hash = match voicing {
        Voicing::Dakuten => dakuten_hash(),
        Voicing::Handakuten => handakuten_hash(),
    };
    let Some(&voiced) = hash.get(&cc) else {
        return;
    };
    let Some(new_char) = transpose(first, cc, voiced) else {
        return;
    };
    let first_byte_len = first.len_utf8();
    let mut buf = [0u8; 4];
    let new_str = new_char.encode_utf8(&mut buf);
    txt.replace_range(0..first_byte_len, new_str);
}
