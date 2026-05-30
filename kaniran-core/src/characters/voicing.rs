//! KanaClass voicing tables, the `dakuten-join` precomposition data,
//! and the four functions that mutate the first character of a kana
//! string. From `characters.lisp:62-104` (tables + `voice-char`) and
//! `:286-314` (`unrendaku` / `rendaku` / `geminate`).
//!
//! [`undakuten_hash`] is the lossy inverse of [`dakuten_hash`] +
//! [`handakuten_hash`] — both `Ba`/`Pa` collapse to `Ha`.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::kana_class::{get_char_class, KanaClass, KANA_CHARACTERS};

/// `*dakuten-hash*` — unvoiced → voiced.
pub fn dakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (Ka, Ga), (Ki, Gi), (Ku, Gu), (Ke, Ge), (Ko, Go),
            (Sa, Za), (Shi, Ji), (Su, Zu), (Se, Ze), (So, Zo),
            (Ta, Da), (Chi, Dji), (Tsu, Dzu), (Te, De), (To, Do),
            (Ha, Ba), (Hi, Bi), (Fu, Bu), (He, Be), (Ho, Bo),
            (U, Vu),
        ]
        .into_iter()
        .collect()
    })
}

/// `*handakuten-hash*` — H-row → P-row.
pub fn handakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [(Ha, Pa), (Hi, Pi), (Fu, Pu), (He, Pe), (Ho, Po)]
            .into_iter()
            .collect()
    })
}

/// `*undakuten-hash*` — voiced or semi-voiced → unvoiced base
/// (lossy: `Ba` and `Pa` both collapse to `Ha`).
pub fn undakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        use KanaClass::*;
        [
            (Ga, Ka), (Gi, Ki), (Gu, Ku), (Ge, Ke), (Go, Ko),
            (Za, Sa), (Ji, Shi), (Zu, Su), (Ze, Se), (Zo, So),
            (Da, Ta), (Dji, Chi), (Dzu, Tsu), (De, Te), (Do, To),
            (Ba, Ha), (Bi, Hi), (Bu, Fu), (Be, He), (Bo, Ho),
            (Pa, Ha), (Pi, Hi), (Pu, Fu), (Pe, He), (Po, Ho),
            (Vu, U),
        ]
        .into_iter()
        .collect()
    })
}

/// `*dakuten-join*` — `(input+combining-mark, precomposed)` pairs for
/// `゛` and `゜`. Built on first access from [`dakuten_hash`] +
/// [`handakuten_hash`] via [`build_dakuten_join`].
pub fn dakuten_join() -> &'static Vec<(String, String)> {
    static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut v = build_dakuten_join(dakuten_hash(), '゛');
        v.extend(build_dakuten_join(handakuten_hash(), '゜'));
        v
    })
}

/// `dakuten-join` (`characters.lisp:93-101`). For each `(unvoiced,
/// voiced)` mapping in `hash`, produce one pair per glyph (hiragana and
/// katakana) of the form `(<unvoiced-glyph><mark>, <voiced-glyph>)`.
///
/// The Lisp returns a flat plist `(in1 out1 in2 out2 ...)`; the Rust
/// port returns paired `Vec<(String, String)>` directly — the only
/// consumer of the Lisp output is [`dakuten_join`], which uses the pairs
/// as alternation entries.
fn build_dakuten_join(hash: &HashMap<KanaClass, KanaClass>, mark: char) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (cc, ccd) in hash.iter() {
        let kc = lookup_kana(*cc).expect("class in *kana-characters*");
        let kcd = lookup_kana(*ccd).expect("class in *kana-characters*");
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

fn lookup_kana(cc: KanaClass) -> Option<&'static str> {
    KANA_CHARACTERS
        .iter()
        .find_map(|(k, s)| if *k == cc { Some(*s) } else { None })
}

/// `voice-char` (`characters.lisp:81-83`). Returns the voiced form of a
/// [`KanaClass`], or the input itself if the class has no voiced
/// counterpart in [`dakuten_hash`]. Only the dakuten mapping is
/// consulted — handakuten (`Ha → Pa` etc.) is not.
///
/// The Lisp idiom `(gethash cc *dakuten-hash* cc)` falls back to the
/// key when missing. Per CONVENTIONS §4.2 the same-typed default
/// collapses directly to `unwrap_or` — no `Option` shape needed,
/// because input and output are both `KanaClass`.
pub fn voice_char(cc: KanaClass) -> KanaClass {
    dakuten_hash().get(&cc).copied().unwrap_or(cc)
}

/// `unrendaku` (`characters.lisp:286-296`). Unvoice the first character
/// of `txt`: maps `がガ → かカ`, `ばバ/ぱパ → はハ`, `ゔヴ → うウ`, etc.
/// via [`undakuten_hash`]. The script (hiragana vs. katakana) of the
/// first glyph is preserved by aligning by index inside the input
/// class's `*kana-characters*` entry.
///
/// Leaves `txt` unchanged when it's empty, when the first character has
/// no [`KanaClass`], or when the class has no unvoiced counterpart.
///
/// The upstream signature is `(txt &key fresh)`. With `:fresh nil`
/// (default) it mutates `txt` in place; with `:fresh t` it copies first
/// and mutates the copy. The Rust port takes `&mut String` and always
/// mutates in place — equivalent to `:fresh nil`. Callers that need
/// `:fresh t` semantics clone before calling.
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

/// `rendaku` voicing argument. The upstream `:handakuten` boolean
/// becomes a 2-variant enum (CONVENTIONS §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voicing {
    Dakuten,
    Handakuten,
}

/// `rendaku` (`characters.lisp:298-309`). Voice the first character of
/// `txt` — `かカ → がガ`, `はハ → ばバ`, etc. With [`Voicing::Handakuten`]
/// the H-row picks up `゜` instead of `゛` (`はハ → ぱパ`); other rows
/// have no handakuten form so the input passes through unchanged. The
/// hiragana/katakana script of the first glyph is preserved.
///
/// Leaves `txt` unchanged when it's empty, when the first character has
/// no [`KanaClass`], or when the class has no voiced counterpart in the
/// chosen hash.
///
/// The upstream signature is `(txt &key fresh handakuten)`. With
/// `:fresh nil` (default) it mutates `txt` in place; with `:fresh t` it
/// copies first and mutates the copy. The Rust port takes `&mut String`
/// and always mutates in place — equivalent to `:fresh nil`. Callers
/// that need `:fresh t` semantics clone before calling. The
/// `:handakuten` boolean becomes a 2-variant [`Voicing`] enum
/// (CONVENTIONS §4.4).
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

/// `geminate` (`characters.lisp:311-314`). Replace the last character of
/// `txt` with the small tsu `っ`. Empty input is left unchanged.
///
/// The upstream signature is `(txt &key fresh)`. With `:fresh nil`
/// (default) it mutates `txt` in place; with `:fresh t` it copies first
/// and mutates the copy. The Rust port takes `&mut String` and always
/// mutates in place — equivalent to `:fresh nil`. Callers that need
/// `:fresh t` semantics clone before calling:
///
/// ```ignore
/// let mut copy = original.to_string();
/// geminate(&mut copy);  // `original` is untouched
/// ```
pub fn geminate(txt: &mut String) {
    if txt.pop().is_some() {
        txt.push('っ');
    }
}

/// Find `c`'s position inside `KANA_CHARACTERS[from]`, then return the
/// glyph at the same position in `KANA_CHARACTERS[to]`. Used by
/// [`unrendaku`] and [`rendaku`] to preserve the hiragana/katakana
/// script of the input.
fn transpose(c: char, from: KanaClass, to: KanaClass) -> Option<char> {
    let from_str = lookup_kana(from)?;
    let to_str = lookup_kana(to)?;
    let pos = from_str.chars().position(|x| x == c)?;
    to_str.chars().nth(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SBCL hash-table iteration order is implementation-defined; the
    /// test sorts both sides before comparing.
    static INTROSPECTED: &[(&str, &str)] = &[
        ("う゛", "ゔ"), ("ウ゛", "ヴ"),
        ("ほ゛", "ぼ"), ("ホ゛", "ボ"),
        ("へ゛", "べ"), ("ヘ゛", "ベ"),
        ("ふ゛", "ぶ"), ("フ゛", "ブ"),
        ("ひ゛", "び"), ("ヒ゛", "ビ"),
        ("は゛", "ば"), ("ハ゛", "バ"),
        ("と゛", "ど"), ("ト゛", "ド"),
        ("て゛", "で"), ("テ゛", "デ"),
        ("つ゛", "づ"), ("ツ゛", "ヅ"),
        ("ち゛", "ぢ"), ("チ゛", "ヂ"),
        ("た゛", "だ"), ("タ゛", "ダ"),
        ("そ゛", "ぞ"), ("ソ゛", "ゾ"),
        ("せ゛", "ぜ"), ("セ゛", "ゼ"),
        ("す゛", "ず"), ("ス゛", "ズ"),
        ("し゛", "じ"), ("シ゛", "ジ"),
        ("さ゛", "ざ"), ("サ゛", "ザ"),
        ("こ゛", "ご"), ("コ゛", "ゴ"),
        ("け゛", "げ"), ("ケ゛", "ゲ"),
        ("く゛", "ぐ"), ("ク゛", "グ"),
        ("き゛", "ぎ"), ("キ゛", "ギ"),
        ("か゛", "が"), ("カ゛", "ガ"),
        ("ほ゜", "ぽ"), ("ホ゜", "ポ"),
        ("へ゜", "ぺ"), ("ヘ゜", "ペ"),
        ("ふ゜", "ぷ"), ("フ゜", "プ"),
        ("ひ゜", "ぴ"), ("ヒ゜", "ピ"),
        ("は゜", "ぱ"), ("ハ゜", "パ"),
    ];

    #[test]
    fn derived_value_matches_introspected_literal_under_sort() {
        let mut derived: Vec<(&str, &str)> = dakuten_join()
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        derived.sort();
        let mut expected: Vec<(&str, &str)> = INTROSPECTED.to_vec();
        expected.sort();
        assert_eq!(derived, expected);
    }
}
