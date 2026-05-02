//! Port of `ichiran/characters:*all-characters*`
//! (`characters.lisp:32-35`).
//!
//! Flat list of `(class, chars)` pairs covering every kana mora /
//! modifier / iteration mark recognized by ichiran. Each `chars` string
//! holds the hiragana form followed by the katakana form, except
//! [`KanaClass::LongVowel`] which has the single character `ー`.
//!
//! Built lazily on first access by appending the four constituents in
//! the same order as the upstream `(append *sokuon-characters*
//! *iteration-characters* *modifier-characters* *kana-characters*)`.
//! A regression test pins the build output to the value the
//! introspector captured — drift in any constituent surfaces there.
//!
//! The `KanaClass` enum lives in [`super::kani_kana_class`] (no Lisp
//! counterpart — see that file for the rationale).

use std::sync::OnceLock;

use super::_star_iteration_characters_star_::ITERATION_CHARACTERS;
use super::_star_kana_characters_star_::KANA_CHARACTERS;
use super::_star_modifier_characters_star_::MODIFIER_CHARACTERS;
use super::_star_sokuon_characters_star_::SOKUON_CHARACTERS;
use super::kani_kana_class::KanaClass;

static CACHE: OnceLock<Vec<(KanaClass, &'static str)>> = OnceLock::new();

pub fn all_characters() -> &'static [(KanaClass, &'static str)] {
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

#[cfg(test)]
mod tests {
    use super::*;
    use KanaClass::*;

    /// Pinned to the value the Lisp introspector captured. Any drift
    /// in the four constituent slices surfaces as a failure here.
    #[test]
    fn matches_introspected_value() {
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
            (A, "あア"),
            (I, "いイ"),
            (U, "うウ"),
            (E, "えエ"),
            (O, "おオ"),
            (Ka, "かカ"),
            (Ki, "きキ"),
            (Ku, "くク"),
            (Ke, "けケ"),
            (Ko, "こコ"),
            (Sa, "さサ"),
            (Shi, "しシ"),
            (Su, "すス"),
            (Se, "せセ"),
            (So, "そソ"),
            (Ta, "たタ"),
            (Chi, "ちチ"),
            (Tsu, "つツ"),
            (Te, "てテ"),
            (To, "とト"),
            (Na, "なナ"),
            (Ni, "にニ"),
            (Nu, "ぬヌ"),
            (Ne, "ねネ"),
            (No, "のノ"),
            (Ha, "はハ"),
            (Hi, "ひヒ"),
            (Fu, "ふフ"),
            (He, "へヘ"),
            (Ho, "ほホ"),
            (Ma, "まマ"),
            (Mi, "みミ"),
            (Mu, "むム"),
            (Me, "めメ"),
            (Mo, "もモ"),
            (Ya, "やヤ"),
            (Yu, "ゆユ"),
            (Yo, "よヨ"),
            (Ra, "らラ"),
            (Ri, "りリ"),
            (Ru, "るル"),
            (Re, "れレ"),
            (Ro, "ろロ"),
            (Wa, "わワ"),
            (Wi, "ゐヰ"),
            (We, "ゑヱ"),
            (Wo, "をヲ"),
            (N, "んン"),
            (Ga, "がガ"),
            (Gi, "ぎギ"),
            (Gu, "ぐグ"),
            (Ge, "げゲ"),
            (Go, "ごゴ"),
            (Za, "ざザ"),
            (Ji, "じジ"),
            (Zu, "ずズ"),
            (Ze, "ぜゼ"),
            (Zo, "ぞゾ"),
            (Da, "だダ"),
            (Dji, "ぢヂ"),
            (Dzu, "づヅ"),
            (De, "でデ"),
            (Do, "どド"),
            (Ba, "ばバ"),
            (Bi, "びビ"),
            (Bu, "ぶブ"),
            (Be, "べベ"),
            (Bo, "ぼボ"),
            (Pa, "ぱパ"),
            (Pi, "ぴピ"),
            (Pu, "ぷプ"),
            (Pe, "ぺペ"),
            (Po, "ぽポ"),
            (Vu, "ゔヴ"),
        ];
        assert_eq!(all_characters(), expected);
    }
}
