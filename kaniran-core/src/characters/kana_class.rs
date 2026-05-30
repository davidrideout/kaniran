//! Kana-class tag enum, the character tables that populate it, and the
//! two lookups derived from those tables. From `characters.lisp:3-53`.
//!
//! The Lisp uses keyword symbols (`:A`, `:KA`, `:SOKUON`, `:+YA`,
//! `:LONG-VOWEL`, …) inline at hundreds of callsites without a named
//! type — the tag set is implicitly defined by the keys of
//! `*all-characters*` plus the same-shape `eql` callsites in
//! `romanize.lisp` and `dict.lisp`. [`KanaClass`] is the closed Rust
//! enumeration that names that set.

use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum KanaClass {
    Sokuon,
    Iter,
    IterV,
    PlusA,
    PlusI,
    PlusU,
    PlusE,
    PlusO,
    PlusYa,
    PlusYu,
    PlusYo,
    PlusWa,
    LongVowel,
    A,
    I,
    U,
    E,
    O,
    Ka,
    Ki,
    Ku,
    Ke,
    Ko,
    Sa,
    Shi,
    Su,
    Se,
    So,
    Ta,
    Chi,
    Tsu,
    Te,
    To,
    Na,
    Ni,
    Nu,
    Ne,
    No,
    Ha,
    Hi,
    Fu,
    He,
    Ho,
    Ma,
    Mi,
    Mu,
    Me,
    Mo,
    Ya,
    Yu,
    Yo,
    Ra,
    Ri,
    Ru,
    Re,
    Ro,
    Wa,
    Wi,
    We,
    Wo,
    N,
    Ga,
    Gi,
    Gu,
    Ge,
    Go,
    Za,
    Ji,
    Zu,
    Ze,
    Zo,
    Da,
    Dji,
    Dzu,
    De,
    Do,
    Ba,
    Bi,
    Bu,
    Be,
    Bo,
    Pa,
    Pi,
    Pu,
    Pe,
    Po,
    Vu,
}

impl KanaClass {
    /// The upstream Lisp keyword's printed name — what `(string :ka)`
    /// would produce: uppercase, no leading colon, hyphenated where the
    /// keyword is. `Ka` → `"KA"`, `PlusYa` → `"+YA"`, `LongVowel` →
    /// `"LONG-VOWEL"`, `IterV` → `"ITER-V"`. Used by
    /// [`long_vowel_modifier_p`], which inspects the last character of
    /// the name to decide whether a preceding glyph's vowel matches a
    /// `+a/+i/+u/+e/+o` modifier.
    pub fn lisp_name(&self) -> &'static str {
        match self {
            KanaClass::Sokuon => "SOKUON",
            KanaClass::Iter => "ITER",
            KanaClass::IterV => "ITER-V",
            KanaClass::PlusA => "+A",
            KanaClass::PlusI => "+I",
            KanaClass::PlusU => "+U",
            KanaClass::PlusE => "+E",
            KanaClass::PlusO => "+O",
            KanaClass::PlusYa => "+YA",
            KanaClass::PlusYu => "+YU",
            KanaClass::PlusYo => "+YO",
            KanaClass::PlusWa => "+WA",
            KanaClass::LongVowel => "LONG-VOWEL",
            KanaClass::A => "A",
            KanaClass::I => "I",
            KanaClass::U => "U",
            KanaClass::E => "E",
            KanaClass::O => "O",
            KanaClass::Ka => "KA",
            KanaClass::Ki => "KI",
            KanaClass::Ku => "KU",
            KanaClass::Ke => "KE",
            KanaClass::Ko => "KO",
            KanaClass::Sa => "SA",
            KanaClass::Shi => "SHI",
            KanaClass::Su => "SU",
            KanaClass::Se => "SE",
            KanaClass::So => "SO",
            KanaClass::Ta => "TA",
            KanaClass::Chi => "CHI",
            KanaClass::Tsu => "TSU",
            KanaClass::Te => "TE",
            KanaClass::To => "TO",
            KanaClass::Na => "NA",
            KanaClass::Ni => "NI",
            KanaClass::Nu => "NU",
            KanaClass::Ne => "NE",
            KanaClass::No => "NO",
            KanaClass::Ha => "HA",
            KanaClass::Hi => "HI",
            KanaClass::Fu => "FU",
            KanaClass::He => "HE",
            KanaClass::Ho => "HO",
            KanaClass::Ma => "MA",
            KanaClass::Mi => "MI",
            KanaClass::Mu => "MU",
            KanaClass::Me => "ME",
            KanaClass::Mo => "MO",
            KanaClass::Ya => "YA",
            KanaClass::Yu => "YU",
            KanaClass::Yo => "YO",
            KanaClass::Ra => "RA",
            KanaClass::Ri => "RI",
            KanaClass::Ru => "RU",
            KanaClass::Re => "RE",
            KanaClass::Ro => "RO",
            KanaClass::Wa => "WA",
            KanaClass::Wi => "WI",
            KanaClass::We => "WE",
            KanaClass::Wo => "WO",
            KanaClass::N => "N",
            KanaClass::Ga => "GA",
            KanaClass::Gi => "GI",
            KanaClass::Gu => "GU",
            KanaClass::Ge => "GE",
            KanaClass::Go => "GO",
            KanaClass::Za => "ZA",
            KanaClass::Ji => "JI",
            KanaClass::Zu => "ZU",
            KanaClass::Ze => "ZE",
            KanaClass::Zo => "ZO",
            KanaClass::Da => "DA",
            KanaClass::Dji => "DJI",
            KanaClass::Dzu => "DZU",
            KanaClass::De => "DE",
            KanaClass::Do => "DO",
            KanaClass::Ba => "BA",
            KanaClass::Bi => "BI",
            KanaClass::Bu => "BU",
            KanaClass::Be => "BE",
            KanaClass::Bo => "BO",
            KanaClass::Pa => "PA",
            KanaClass::Pi => "PI",
            KanaClass::Pu => "PU",
            KanaClass::Pe => "PE",
            KanaClass::Po => "PO",
            KanaClass::Vu => "VU",
        }
    }
}

/// `*sokuon-characters*` — geminating mark (small tsu).
pub static SOKUON_CHARACTERS: &[(KanaClass, &str)] = &[(KanaClass::Sokuon, "っッ")];

/// `*iteration-characters*` — plain and voiced iteration marks.
pub static ITERATION_CHARACTERS: &[(KanaClass, &str)] = &[
    (KanaClass::Iter, "ゝヽ"),
    (KanaClass::IterV, "ゞヾ"),
];

/// `*modifier-characters*` — small-form vowels, y-glides, long-vowel
/// mark.
pub static MODIFIER_CHARACTERS: &[(KanaClass, &str)] = &[
    (KanaClass::PlusA, "ぁァ"),
    (KanaClass::PlusI, "ぃィ"),
    (KanaClass::PlusU, "ぅゥ"),
    (KanaClass::PlusE, "ぇェ"),
    (KanaClass::PlusO, "ぉォ"),
    (KanaClass::PlusYa, "ゃャ"),
    (KanaClass::PlusYu, "ゅュ"),
    (KanaClass::PlusYo, "ょョ"),
    (KanaClass::PlusWa, "ゎヮ"),
    (KanaClass::LongVowel, "ー"),
];

/// `*kana-characters*` — every regular mora, hiragana then katakana.
pub static KANA_CHARACTERS: &[(KanaClass, &str)] = &[
    (KanaClass::A, "あア"),
    (KanaClass::I, "いイ"),
    (KanaClass::U, "うウ"),
    (KanaClass::E, "えエ"),
    (KanaClass::O, "おオ"),
    (KanaClass::Ka, "かカ"),
    (KanaClass::Ki, "きキ"),
    (KanaClass::Ku, "くク"),
    (KanaClass::Ke, "けケ"),
    (KanaClass::Ko, "こコ"),
    (KanaClass::Sa, "さサ"),
    (KanaClass::Shi, "しシ"),
    (KanaClass::Su, "すス"),
    (KanaClass::Se, "せセ"),
    (KanaClass::So, "そソ"),
    (KanaClass::Ta, "たタ"),
    (KanaClass::Chi, "ちチ"),
    (KanaClass::Tsu, "つツ"),
    (KanaClass::Te, "てテ"),
    (KanaClass::To, "とト"),
    (KanaClass::Na, "なナ"),
    (KanaClass::Ni, "にニ"),
    (KanaClass::Nu, "ぬヌ"),
    (KanaClass::Ne, "ねネ"),
    (KanaClass::No, "のノ"),
    (KanaClass::Ha, "はハ"),
    (KanaClass::Hi, "ひヒ"),
    (KanaClass::Fu, "ふフ"),
    (KanaClass::He, "へヘ"),
    (KanaClass::Ho, "ほホ"),
    (KanaClass::Ma, "まマ"),
    (KanaClass::Mi, "みミ"),
    (KanaClass::Mu, "むム"),
    (KanaClass::Me, "めメ"),
    (KanaClass::Mo, "もモ"),
    (KanaClass::Ya, "やヤ"),
    (KanaClass::Yu, "ゆユ"),
    (KanaClass::Yo, "よヨ"),
    (KanaClass::Ra, "らラ"),
    (KanaClass::Ri, "りリ"),
    (KanaClass::Ru, "るル"),
    (KanaClass::Re, "れレ"),
    (KanaClass::Ro, "ろロ"),
    (KanaClass::Wa, "わワ"),
    (KanaClass::Wi, "ゐヰ"),
    (KanaClass::We, "ゑヱ"),
    (KanaClass::Wo, "をヲ"),
    (KanaClass::N, "んン"),
    (KanaClass::Ga, "がガ"),
    (KanaClass::Gi, "ぎギ"),
    (KanaClass::Gu, "ぐグ"),
    (KanaClass::Ge, "げゲ"),
    (KanaClass::Go, "ごゴ"),
    (KanaClass::Za, "ざザ"),
    (KanaClass::Ji, "じジ"),
    (KanaClass::Zu, "ずズ"),
    (KanaClass::Ze, "ぜゼ"),
    (KanaClass::Zo, "ぞゾ"),
    (KanaClass::Da, "だダ"),
    (KanaClass::Dji, "ぢヂ"),
    (KanaClass::Dzu, "づヅ"),
    (KanaClass::De, "でデ"),
    (KanaClass::Do, "どド"),
    (KanaClass::Ba, "ばバ"),
    (KanaClass::Bi, "びビ"),
    (KanaClass::Bu, "ぶブ"),
    (KanaClass::Be, "べベ"),
    (KanaClass::Bo, "ぼボ"),
    (KanaClass::Pa, "ぱパ"),
    (KanaClass::Pi, "ぴピ"),
    (KanaClass::Pu, "ぷプ"),
    (KanaClass::Pe, "ぺペ"),
    (KanaClass::Po, "ぽポ"),
    (KanaClass::Vu, "ゔヴ"),
];

/// `*all-characters*` — `(append sokuon iteration modifier kana)`.
pub fn all_characters() -> &'static [(KanaClass, &'static str)] {
    static CACHE: OnceLock<Vec<(KanaClass, &'static str)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut v = Vec::with_capacity(
            SOKUON_CHARACTERS.len()
                + ITERATION_CHARACTERS.len()
                + MODIFIER_CHARACTERS.len()
                + KANA_CHARACTERS.len(),
        );
        v.extend_from_slice(SOKUON_CHARACTERS);
        v.extend_from_slice(ITERATION_CHARACTERS);
        v.extend_from_slice(MODIFIER_CHARACTERS);
        v.extend_from_slice(KANA_CHARACTERS);
        v
    })
}

/// `*char-class-hash*` — per-glyph reverse lookup into [`KanaClass`].
pub fn char_class_hash() -> &'static HashMap<char, KanaClass> {
    static CACHE: OnceLock<HashMap<char, KanaClass>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut h = HashMap::new();
        for (class, chars) in all_characters() {
            for c in chars.chars() {
                h.insert(c, *class);
            }
        }
        h
    })
}

/// `get-char-class` (`characters.lisp:44-45`) — lookup into
/// [`char_class_hash`]. Lisp returns the input char on a miss; per
/// CONVENTIONS §4.2 the Rust port returns `Option<KanaClass>` and
/// lets the caller fall back to the input it already has.
pub fn get_char_class(c: char) -> Option<KanaClass> {
    char_class_hash().get(&c).copied()
}

/// `long-vowel-modifier-p` (`characters.lisp:47-53`) — true when a small
/// modifier glyph (`ぁ ィ ぅ ェ ぉ`, classified as `+A/+I/+U/+E/+O`)
/// extends the preceding character's vowel — e.g. `か` followed by `ぁ`
/// produces a long `aa` rather than a `kya`-style fused mora. The check
/// compares the modifier's vowel target against the last character of
/// the previous glyph's `KanaClass` keyword name (e.g. `Ka` → `"KA"`,
/// last char `'A'`).
///
/// Returns `false` when `modifier` isn't one of the five `+vowel`
/// variants, or when `prev_char` has no known [`KanaClass`]. The Lisp's
/// `(keywordp char-class)` guard is subsumed by [`get_char_class`]
/// returning `Option<KanaClass>` (CONVENTIONS §4.2).
pub fn long_vowel_modifier_p(modifier: KanaClass, prev_char: char) -> bool {
    let vowel = match modifier {
        KanaClass::PlusA => 'A',
        KanaClass::PlusI => 'I',
        KanaClass::PlusU => 'U',
        KanaClass::PlusE => 'E',
        KanaClass::PlusO => 'O',
        _ => return false,
    };
    let Some(class) = get_char_class(prev_char) else {
        return false;
    };
    class.lisp_name().chars().last() == Some(vowel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use KanaClass::*;

    /// Pinned against the Lisp introspector's captured value.
    #[test]
    fn all_characters_matches_introspected_value() {
        let expected: &[(KanaClass, &str)] = &[
            (Sokuon, "っッ"),
            (Iter, "ゝヽ"),
            (IterV, "ゞヾ"),
            (PlusA, "ぁァ"),
            (PlusI, "ぃィ"),
            (PlusU, "ぅゥ"),
            (PlusE, "ぇェ"),
            (PlusO, "ぉォ"),
            (PlusYa, "ゃャ"),
            (PlusYu, "ゅュ"),
            (PlusYo, "ょョ"),
            (PlusWa, "ゎヮ"),
            (LongVowel, "ー"),
            (A, "あア"), (I, "いイ"), (U, "うウ"), (E, "えエ"), (O, "おオ"),
            (Ka, "かカ"), (Ki, "きキ"), (Ku, "くク"), (Ke, "けケ"), (Ko, "こコ"),
            (Sa, "さサ"), (Shi, "しシ"), (Su, "すス"), (Se, "せセ"), (So, "そソ"),
            (Ta, "たタ"), (Chi, "ちチ"), (Tsu, "つツ"), (Te, "てテ"), (To, "とト"),
            (Na, "なナ"), (Ni, "にニ"), (Nu, "ぬヌ"), (Ne, "ねネ"), (No, "のノ"),
            (Ha, "はハ"), (Hi, "ひヒ"), (Fu, "ふフ"), (He, "へヘ"), (Ho, "ほホ"),
            (Ma, "まマ"), (Mi, "みミ"), (Mu, "むム"), (Me, "めメ"), (Mo, "もモ"),
            (Ya, "やヤ"), (Yu, "ゆユ"), (Yo, "よヨ"),
            (Ra, "らラ"), (Ri, "りリ"), (Ru, "るル"), (Re, "れレ"), (Ro, "ろロ"),
            (Wa, "わワ"), (Wi, "ゐヰ"), (We, "ゑヱ"), (Wo, "をヲ"),
            (N, "んン"),
            (Ga, "がガ"), (Gi, "ぎギ"), (Gu, "ぐグ"), (Ge, "げゲ"), (Go, "ごゴ"),
            (Za, "ざザ"), (Ji, "じジ"), (Zu, "ずズ"), (Ze, "ぜゼ"), (Zo, "ぞゾ"),
            (Da, "だダ"), (Dji, "ぢヂ"), (Dzu, "づヅ"), (De, "でデ"), (Do, "どド"),
            (Ba, "ばバ"), (Bi, "びビ"), (Bu, "ぶブ"), (Be, "べベ"), (Bo, "ぼボ"),
            (Pa, "ぱパ"), (Pi, "ぴピ"), (Pu, "ぷプ"), (Pe, "ぺペ"), (Po, "ぽポ"),
            (Vu, "ゔヴ"),
        ];
        assert_eq!(all_characters(), expected);
    }

    /// 173 entries per the Lisp introspector.
    #[test]
    fn char_class_hash_build_logic_produces_173_entries() {
        assert_eq!(char_class_hash().len(), 173);
    }
}
