//! String / regex constants from `characters.lisp:106-129`.

/// `*abnormal-chars*` — source side of the abnormal→normal map,
/// paired index-by-index with [`NORMAL_CHARS`].
pub static ABNORMAL_CHARS: &str = "\
０１２３４５６７８９\
ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ\
ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ\
＃＄％＆（）＊＋／〈＝〉？＠［］＾＿‘｛｜｝～\
･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// `*normal-chars*` — target side of the abnormal→normal map.
/// Upstream `(concatenate 'string <ASCII prefix> *full-width-kana*)`;
/// captured as a literal here.
pub static NORMAL_CHARS: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#$%&()*+/<=>?@[]^_`{|}~・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// `*full-width-kana*` — paired index-by-index with [`HALF_WIDTH_KANA`].
pub static FULL_WIDTH_KANA: &str =
    "・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// `*half-width-kana*` — paired index-by-index with [`FULL_WIDTH_KANA`].
pub static HALF_WIDTH_KANA: &str =
    "･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// `*decimal-point-regex*`.
pub static DECIMAL_POINT_REGEX: &str = "[.,]";

/// `*digit-regex*` — ASCII, full-width Latin, or ideographic zero `〇`.
pub static DIGIT_REGEX: &str = "[0-9０-９〇]";

/// `*hiragana-regex*` — hiragana + iteration marks `ゝ ゞ` + long-vowel `ー`.
pub static HIRAGANA_REGEX: &str = "[ぁ-ゔゝゞー]";

/// `*kanji-regex*` — CJK ideograph block + `々` + abbreviation marks `ヶ 〆`.
pub static KANJI_REGEX: &str = "[々ヶ〆一-龯]";

/// `*kanji-char-regex*` — CJK ideograph block only (no `々ヶ〆`).
pub static KANJI_CHAR_REGEX: &str = "[一-龯]";

/// `*katakana-regex*` — katakana + iteration marks `ヽ ヾ` + long-vowel `ー`.
pub static KATAKANA_REGEX: &str = "[ァ-ヺヽヾー]";

/// `*katakana-uniq-regex*` — [`KATAKANA_REGEX`] without `ー`.
pub static KATAKANA_UNIQ_REGEX: &str = "[ァ-ヺヽヾ]";

/// `*nonword-regex*` — complement of [`WORD_REGEX`].
pub static NONWORD_REGEX: &str = "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";

/// `*numeric-regex*` — ASCII + full-width digits + ideographic zero +
/// kanji numerals (`一-九` plus traditional / large-unit forms).
pub static NUMERIC_REGEX: &str = "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]";

/// `*num-word-regex*` — [`WORD_REGEX`] plus digits.
pub static NUM_WORD_REGEX: &str = "[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]";

/// `*word-regex*` — kanji + iteration/abbreviation marks + katakana +
/// hiragana + ideographic zero.
pub static WORD_REGEX: &str = "[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";

/// `*punctuation-marks*` (`characters.lisp:85-91`) —
/// `(japanese, ascii-equivalent)` pairs for romanization.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Abnormal/normal pairing is index-by-index; lengths must match.
    #[test]
    fn abnormal_and_normal_have_equal_char_count() {
        assert_eq!(ABNORMAL_CHARS.chars().count(), NORMAL_CHARS.chars().count());
    }

    /// Same for the kana-context source/target.
    #[test]
    fn half_and_full_width_kana_have_equal_char_count() {
        assert_eq!(
            HALF_WIDTH_KANA.chars().count(),
            FULL_WIDTH_KANA.chars().count()
        );
    }
}
