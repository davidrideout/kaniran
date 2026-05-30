//! Kana-class character tables and the two lookups derived from them,
//! from `characters.lisp:3-37`.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::kani_kana_class::KanaClass;

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
