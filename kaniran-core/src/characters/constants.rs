//! String / regex constants for the `characters` module.
//!
//! Each `_REGEX` constant is a pattern string consumed by the lazily-
//! compiled scanners in [`super::char_class_scanners`] /
//! [`super::_star_char_scanners_star_`] / [`super::_star_basic_split_regex_star_`].
//! `ABNORMAL_CHARS`, `FULL_WIDTH_KANA`, and `HALF_WIDTH_KANA` are the
//! source/target character-class strings consumed by
//! [`super::to_normal_char`] and (transitively) [`super::_star_normal_chars_star_`].
//!
//! Originally one Lisp `defparameter` per file under
//! `ichiran/characters` (`characters.lisp:106-129`); consolidated here
//! during phase 2 cleanup.

/// `*abnormal-chars*` — source side of the abnormal→normal map:
/// full-width ASCII printables and half-width katakana, paired
/// index-by-index with [`normal_chars`][super::_star_normal_chars_star_::normal_chars].
pub static ABNORMAL_CHARS: &str = "\
０１２３４５６７８９\
ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ\
ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ\
＃＄％＆（）＊＋／〈＝〉？＠［］＾＿‘｛｜｝～\
･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// `*full-width-kana*` — 59-character string of full-width katakana
/// glyphs, paired index-by-index with [`HALF_WIDTH_KANA`].
pub static FULL_WIDTH_KANA: &str =
    "・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// `*half-width-kana*` — 59-character string of half-width katakana
/// glyphs, paired index-by-index with [`FULL_WIDTH_KANA`].
pub static HALF_WIDTH_KANA: &str =
    "･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";

/// `*decimal-point-regex*` — `[.,]`. One of the small parts composed
/// into `*basic-split-regex*`.
pub static DECIMAL_POINT_REGEX: &str = "[.,]";

/// `*digit-regex*` — one digit in ASCII, full-width Latin, or the
/// ideographic-zero glyph `〇`.
pub static DIGIT_REGEX: &str = "[0-9０-９〇]";

/// `*hiragana-regex*` — one hiragana code point, including the
/// iteration marks `ゝ ゞ` and the long-vowel marker `ー`.
pub static HIRAGANA_REGEX: &str = "[ぁ-ゔゝゞー]";

/// `*kanji-regex*` — one kanji-ish code point: the CJK ideograph block
/// plus the iteration mark `々` and the abbreviation marks `ヶ 〆`.
pub static KANJI_REGEX: &str = "[々ヶ〆一-龯]";

/// `*kanji-char-regex*` — strict version of [`KANJI_REGEX`] limited to
/// the U+4E00..U+9FAF CJK Unified Ideograph range (excludes `々ヶ〆`).
pub static KANJI_CHAR_REGEX: &str = "[一-龯]";

/// `*katakana-regex*` — one katakana code point, including the
/// iteration marks `ヽ ヾ` and the long-vowel marker `ー`.
pub static KATAKANA_REGEX: &str = "[ァ-ヺヽヾー]";

/// `*katakana-uniq-regex*` — like [`KATAKANA_REGEX`] but excludes the
/// long-vowel marker `ー` (shared with hiragana).
pub static KATAKANA_UNIQ_REGEX: &str = "[ァ-ヺヽヾ]";

/// `*nonword-regex*` — complement of [`WORD_REGEX`].
pub static NONWORD_REGEX: &str = "[^々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";

/// `*numeric-regex*` — one numeric character: ASCII digit, full-width
/// digit, ideographic zero `〇`, kanji-numeral `一-九`, or one of the
/// traditional / large-unit numeral kanji.
pub static NUMERIC_REGEX: &str = "[0-9０-９〇一二三四五六七八九零壱弐参拾十百千万億兆京]";

/// `*num-word-regex*` — one character that can appear inside a mixed
/// number-and-word token. Differs from [`WORD_REGEX`] only by also
/// admitting digits.
pub static NUM_WORD_REGEX: &str = "[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]";

/// `*word-regex*` — one character inside a Japanese word: kanji +
/// iteration/abbreviation marks + katakana + hiragana + ideographic
/// zero.
pub static WORD_REGEX: &str = "[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]";
