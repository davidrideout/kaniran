use super::*;
use KanaClass::*;

// --- _star_all_characters_star_ ---
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

// --- _star_char_class_hash_star_ ---
#[test]
fn build_logic_produces_173_entries() {
    assert_eq!(char_class_hash().len(), 173);
}

// --- _star_dakuten_join_star_ ---
static INTROSPECTED: &[(&str, &str)] = &[
    ("う゛", "ゔ"),
    ("ウ゛", "ヴ"),
    ("ほ゛", "ぼ"),
    ("ホ゛", "ボ"),
    ("へ゛", "べ"),
    ("ヘ゛", "ベ"),
    ("ふ゛", "ぶ"),
    ("フ゛", "ブ"),
    ("ひ゛", "び"),
    ("ヒ゛", "ビ"),
    ("は゛", "ば"),
    ("ハ゛", "バ"),
    ("と゛", "ど"),
    ("ト゛", "ド"),
    ("て゛", "で"),
    ("テ゛", "デ"),
    ("つ゛", "づ"),
    ("ツ゛", "ヅ"),
    ("ち゛", "ぢ"),
    ("チ゛", "ヂ"),
    ("た゛", "だ"),
    ("タ゛", "ダ"),
    ("そ゛", "ぞ"),
    ("ソ゛", "ゾ"),
    ("せ゛", "ぜ"),
    ("セ゛", "ゼ"),
    ("す゛", "ず"),
    ("ス゛", "ズ"),
    ("し゛", "じ"),
    ("シ゛", "ジ"),
    ("さ゛", "ざ"),
    ("サ゛", "ザ"),
    ("こ゛", "ご"),
    ("コ゛", "ゴ"),
    ("け゛", "げ"),
    ("ケ゛", "ゲ"),
    ("く゛", "ぐ"),
    ("ク゛", "グ"),
    ("き゛", "ぎ"),
    ("キ゛", "ギ"),
    ("か゛", "が"),
    ("カ゛", "ガ"),
    ("ほ゜", "ぽ"),
    ("ホ゜", "ポ"),
    ("へ゜", "ぺ"),
    ("ヘ゜", "ペ"),
    ("ふ゜", "ぷ"),
    ("フ゜", "プ"),
    ("ひ゜", "ぴ"),
    ("ヒ゜", "ピ"),
    ("は゜", "ぱ"),
    ("ハ゜", "パ"),
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

// --- _star_normal_chars_star_ ---
/// Asserts the concatenated string equals the introspected value.
#[test]
fn normal_chars_matches_introspected_value() {
    assert_eq!(
        normal_chars(),
        "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜"
    );
}

// --- _star_basic_split_regex_star_ ---
#[test]
fn basic_split_regex_compiles_under_fancy_regex() {
    fancy_regex::Regex::new(basic_split_regex()).expect("regex must compile");
}

#[test]
fn basic_split_regex_matches_introspected_value() {
    assert_eq!(
        basic_split_regex(),
        "((?:(?<![.,]|[0-9０-９〇])[0-9０-９〇]+|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]*[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])"
    );
}

// --- _star_char_scanners_star_ ---
#[test]
fn char_scanners_compiles_under_fancy_regex() {
    let h = char_scanners();
    for (class, _) in CHAR_CLASS_REGEX_MAPPING {
        assert!(h.contains_key(class), "missing scanner for {class:?}");
    }
}

// --- _star_char_scanners_inner_star_ ---
#[test]
fn char_scanners_inner_compiles_under_fancy_regex() {
    let h = char_scanners_inner();
    for (class, _) in CHAR_CLASS_REGEX_MAPPING {
        assert!(h.contains_key(class), "missing scanner for {class:?}");
    }
}
