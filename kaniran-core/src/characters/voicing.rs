use super::char_class::get_char_class;
use super::constants::KANA_CHARACTERS;
use super::helpers::{dakuten_hash, handakuten_hash, undakuten_hash};
use super::kani_kana_class::KanaClass;
use std::collections::HashMap;

/// Port of `ichiran/characters:voice-char` (`characters.lisp:81-83`).
///
/// Returns the voiced form of a [`KanaClass`], or the input itself if
/// the class has no voiced counterpart in
/// [`super::helpers::dakuten_hash`]. Only the dakuten
/// mapping is consulted — handakuten (`Ha → Pa` etc.) is not.
pub fn voice_char(cc: KanaClass) -> KanaClass {
    dakuten_hash().get(&cc).copied().unwrap_or(cc)
}

/// Port of `ichiran/characters:dakuten-join` (`characters.lisp:93-101`).
///
/// Build `(<unvoiced-glyph><mark>, <voiced-glyph>)` pairs for every
/// glyph (hiragana and katakana) of each `(unvoiced, voiced)` mapping
/// in `hash`.
pub fn dakuten_join(hash: &HashMap<KanaClass, KanaClass>, mark: char) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (cc, ccd) in hash.iter() {
        let kc = lookup(*cc);
        let kcd = lookup(*ccd);
        let kc_chars: Vec<char> = kc.chars().collect();
        let kcd_chars: Vec<char> = kcd.chars().collect();
        let offset = kc_chars.len().saturating_sub(kcd_chars.len());
        let kc_aligned = &kc_chars[offset..];
        for (i, c1) in kc_aligned.iter().enumerate() {
            let c2 = kcd_chars[i];
            let mut input = String::new();
            input.push(*c1);
            input.push(mark);
            let output: String = std::iter::once(c2).collect();
            out.push((input, output));
        }
    }
    out
}

fn lookup(cc: KanaClass) -> &'static str {
    KANA_CHARACTERS
        .iter()
        .find_map(|(k, s)| if *k == cc { Some(*s) } else { None })
        .expect("class in *kana-characters*")
}

/// Port of `ichiran/characters:unrendaku` (`characters.lisp:286-296`).
///
/// Unvoice the first character of `txt`: maps `がガ → かカ`,
/// `ばバ/ぱパ → はハ`, `ゔヴ → うウ`, etc. The script (hiragana vs.
/// katakana) of the first glyph is preserved by aligning by index
/// inside the input class's `*kana-characters*` entry.
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
/// glyph at the same position in `KANA_CHARACTERS[to]`, preserving the
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

/// Port of `ichiran/characters:rendaku` (`characters.lisp:298-309`).
///
/// Voice the first character of `txt` — `かカ → がガ`, `はハ → ばバ`,
/// etc. With [`Voicing::Handakuten`] the H-row picks up `゜` instead of
/// `゛` (`はハ → ぱパ`); other rows have no handakuten form so the
/// input passes through unchanged. The hiragana/katakana script of the
/// first glyph is preserved.
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

/// Port of `ichiran/characters:geminate` (`characters.lisp:311-314`).
///
/// Replace the last character of `txt` with the small tsu `っ`. Empty
/// input is left unchanged.
pub fn geminate(txt: &mut String) {
    if txt.pop().is_some() {
        txt.push('っ');
    }
}
