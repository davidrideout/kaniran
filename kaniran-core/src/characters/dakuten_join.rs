//! Port of `ichiran/characters:dakuten-join` (`characters.lisp:93-101`).
//!
//! Build `(<unvoiced-glyph><mark>, <voiced-glyph>)` pairs for every
//! glyph (hiragana and katakana) of each `(unvoiced, voiced)` mapping
//! in `hash`.

use std::collections::HashMap;

use super::_star_kana_characters_star_::KANA_CHARACTERS;
use super::kani_kana_class::KanaClass;

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
