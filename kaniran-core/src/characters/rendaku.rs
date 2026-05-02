//! Port of `ichiran/characters:rendaku` (`characters.lisp:298-309`).
//!
//! Voice the first character of `txt` — `かカ → がガ`, `はハ → ばバ`,
//! etc. With [`Voicing::Handakuten`] the H-row picks up `゜` instead of
//! `゛` (`はハ → ぱパ`); other rows have no handakuten form so the
//! input passes through unchanged. The hiragana/katakana script of the
//! first glyph is preserved.
//!
//! Returns `txt` unchanged when it's empty, when the first character
//! has no [`KanaClass`], or when the class has no voiced counterpart
//! in the chosen hash. The Lisp's `&key handakuten` boolean becomes a
//! 2-variant enum (CONVENTIONS §4.4); `&key fresh` is dropped — the
//! port always allocates (CONVENTIONS §4.6).

use super::_star_dakuten_hash_star_::dakuten_hash;
use super::_star_handakuten_hash_star_::handakuten_hash;
use super::get_char_class::get_char_class;
use super::unrendaku::transpose;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voicing {
    Dakuten,
    Handakuten,
}

pub fn rendaku(txt: &str, voicing: Voicing) -> String {
    let mut chars: Vec<char> = txt.chars().collect();
    let Some(&first) = chars.first() else {
        return String::new();
    };
    let Some(cc) = get_char_class(first) else {
        return txt.to_string();
    };
    let hash = match voicing {
        Voicing::Dakuten => dakuten_hash(),
        Voicing::Handakuten => handakuten_hash(),
    };
    let Some(&voiced) = hash.get(&cc) else {
        return txt.to_string();
    };
    let Some(new_char) = transpose(first, cc, voiced) else {
        return txt.to_string();
    };
    chars[0] = new_char;
    chars.into_iter().collect()
}
