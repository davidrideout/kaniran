//! Port of `ichiran/characters:dakuten-join` (`characters.lisp:93-101`).
//!
//! Build a list of `(input-with-combining-mark, single-precomposed-char)`
//! pairs: for each `(unvoiced, voiced)` mapping in `hash`, produce one
//! pair per glyph (hiragana and katakana) of the form
//! `(<unvoiced-glyph><mark>, <voiced-glyph>)`.
//!
//! The Lisp returns a flat plist `(in1 out1 in2 out2 ...)`; the Rust
//! port returns paired `Vec<(String, String)>` directly — the only
//! consumer of the Lisp output is
//! [`super::_star_dakuten_join_star_::dakuten_join`], which uses the
//! pairs as alternation entries.

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
