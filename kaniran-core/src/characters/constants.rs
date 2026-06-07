use super::char_class::CharClass;
use super::kani_kana_class::KanaClass;

/// Port of `ichiran/characters:*sokuon-characters*`
/// (`characters.lisp:3`).
///
/// Single-entry table for the geminating mark (small tsu, hiragana
/// `っ` / katakana `ッ`).
pub static SOKUON_CHARACTERS: &[(KanaClass, &str)] = &[(KanaClass::Sokuon, "っッ")];

/// Port of `ichiran/characters:*iteration-characters*`
/// (`characters.lisp:5`).
///
/// The two iteration marks: plain (`ゝヽ`) and voiced (`ゞヾ`).
pub static ITERATION_CHARACTERS: &[(KanaClass, &str)] =
    &[(KanaClass::Iter, "ゝヽ"), (KanaClass::IterV, "ゞヾ")];

/// Port of `ichiran/characters:*modifier-characters*`
/// (`characters.lisp:7-9`).
///
/// Small-form vowels and y-glides used as modifiers (e.g. `ぁ` in
/// `きゃ`), plus the long-vowel mark `ー`. 10 entries.
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

/// Port of `ichiran/characters:*kana-characters*`
/// (`characters.lisp:11-30`).
///
/// All regular morae — vowels, k/s/t/n/h/m/y/r/w-rows, `n`, the voiced
/// and semi-voiced rows, and `vu`. 74 entries. Each value pairs the
/// hiragana glyph with the katakana glyph in that order.
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

/// Port of `ichiran/characters:*punctuation-marks*`
/// (`characters.lisp:85-91`).
///
/// Pairs of `(japanese-mark, ascii-equivalent)` used to romanize
/// punctuation. 18 entries.
pub static PUNCTUATION_MARKS: &[(&str, &str)] = &[
    ("【", " ["),
    ("】", "] "),
    ("、", ", "),
    ("，", ", "),
    ("。", ". "),
    ("・・・", "... "),
    ("・", " "),
    ("　", " "),
    ("「", " \""),
    ("」", "\" "),
    ("゛", "\""),
    ("『", " «"),
    ("』", "» "),
    ("〜", " - "),
    ("：", ": "),
    ("！", "! "),
    ("？", "? "),
    ("；", "; "),
];

/// Port of `ichiran/characters:*half-width-kana*`
/// (`characters.lisp:106`).
///
/// 59-character string of half-width katakana glyphs, paired
/// index-by-index with [`FULL_WIDTH_KANA`][super::constants::FULL_WIDTH_KANA].
pub static HALF_WIDTH_KANA: &str = "･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// Port of `ichiran/characters:*full-width-kana*`
/// (`characters.lisp:107`).
///
/// 59-character string of full-width katakana glyphs, paired
/// index-by-index with [`HALF_WIDTH_KANA`][super::constants::HALF_WIDTH_KANA].
pub static FULL_WIDTH_KANA: &str =
    "・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// Port of `ichiran/characters:*abnormal-chars*`
/// (`characters.lisp:109`).
///
/// Source side of the abnormal→normal character map: full-width ASCII
/// printables and half-width katakana.
pub static ABNORMAL_CHARS: &str = "\
０１２３４５６７８９\
ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ\
ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ\
＃＄％＆（）＊＋／〈＝〉？＠［］＾＿‘｛｜｝～\
･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// Port of `ichiran/characters:*katakana-regex*`
/// (`characters.lisp:118`).
///
/// Matches one katakana code point, including the iteration marks
/// ヽ ヾ and the long-vowel marker ー.
pub static KATAKANA_REGEX: &str = "[ァ-ヺヽヾー]";

/// Port of `ichiran/characters:*katakana-uniq-regex*`
/// (`characters.lisp:119`).
///
/// Matches one katakana code point, excluding the long-vowel marker ー
/// (which is shared with hiragana and so isn't unique to katakana).
pub static KATAKANA_UNIQ_REGEX: &str = "[ァ-ヺヽヾ]";

/// Port of `ichiran/characters:*hiragana-regex*`
/// (`characters.lisp:120`).
///
/// Matches one hiragana code point, including the iteration marks
/// ゝ ゞ and the long-vowel marker ー.
pub static HIRAGANA_REGEX: &str = "[ぁ-ゔゝゞー]";

/// Port of `ichiran/characters:*kanji-regex*`
/// (`characters.lisp:121`).
///
/// Matches one kanji-ish code point: the CJK ideograph block
/// plus the iteration mark 々 and the abbreviation marks ヶ 〆.
pub static KANJI_REGEX: &str = "[々ヶ〆一-龯]";

/// Port of `ichiran/characters:*kanji-char-regex*`
/// (`characters.lisp:122`).
///
/// Matches one CJK Unified Ideograph in the U+4E00…U+9FAF range;
/// excludes the iteration mark 々 and the abbreviation marks ヶ 〆.
pub static KANJI_CHAR_REGEX: &str = "[一-龯]";

/// Port of `ichiran/characters:*nonword-regex*`
/// (`characters.lisp:124`).
///
/// Matches one character that is NOT part of a word as ichiran
/// defines it: complement of kanji + iteration/abbreviation marks
/// + katakana + hiragana + ideographic zero.
pub static NONWORD_REGEX: &str = "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";

/// Port of `ichiran/characters:*numeric-regex*`
/// (`characters.lisp:125`).
///
/// Matches one numeric character: ASCII digit, full-width digit,
/// ideographic zero 〇, kanji-numeral (一-九), or one of the
/// traditional / large-unit numeral kanji used in Japanese
/// number-writing (零壱弐参拾十百千万億兆京).
pub static NUMERIC_REGEX: &str = "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]";

/// Port of `ichiran/characters:*num-word-regex*`
/// (`characters.lisp:126`).
///
/// Matches one character that can appear inside a mixed
/// number-and-word token: digits (ASCII or full-width), 〇, kanji,
/// katakana, or hiragana.
pub static NUM_WORD_REGEX: &str = "[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]";

/// Port of `ichiran/characters:*word-regex*`
/// (`characters.lisp:127`).
///
/// Matches one character that can appear inside a Japanese word:
/// kanji + iteration/abbreviation marks + katakana + hiragana +
/// ideographic zero.
pub static WORD_REGEX: &str = "[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";

/// Port of `ichiran/characters:*digit-regex*`
/// (`characters.lisp:128`).
///
/// Matches one digit in either ASCII, full-width Latin, or the
/// single ideographic-zero glyph 〇.
pub static DIGIT_REGEX: &str = "[0-9０-９〇]";

/// Port of `ichiran/characters:*decimal-point-regex*`
/// (`characters.lisp:129`).
///
/// Matches a decimal-point separator (`.` or `,`).
pub static DECIMAL_POINT_REGEX: &str = "[.,]";

/// Port of `ichiran/characters:*char-class-regex-mapping*`
/// (`characters.lisp:136`).
///
/// Mapping from a [`CharClass`] to a regex string that matches one
/// character of that class.
pub static CHAR_CLASS_REGEX_MAPPING: &[(CharClass, &str)] = &[
    (CharClass::Katakana, "[ァ-ヺヽヾー]"),
    (CharClass::KatakanaUniq, "[ァ-ヺヽヾ]"),
    (CharClass::Hiragana, "[ぁ-ゔゝゞー]"),
    (CharClass::Kanji, "[々ヶ〆一-龯]"),
    (CharClass::KanjiChar, "[一-龯]"),
    (CharClass::Kana, "([ァ-ヺヽヾー]|[ぁ-ゔゝゞー])"),
    (CharClass::Traditional, "([ぁ-ゔゝゞー]|[々ヶ〆一-龯])"),
    (CharClass::Nonword, "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]"),
    (
        CharClass::Number,
        "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]",
    ),
];

#[cfg(test)]
mod tests;
