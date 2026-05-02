//! Port of `ichiran/characters:*full-width-kana*`
//! (`characters.lisp:107`).
//!
//! 59-character string of full-width katakana glyphs, paired
//! index-by-index with [`HALF_WIDTH_KANA`][super::_star_half_width_kana_star_::HALF_WIDTH_KANA]
//! to translate the half-width forms into their canonical full-width
//! equivalents. Also forms the kana suffix of `*normal-chars*`.

pub static FULL_WIDTH_KANA: &str =
    "・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";
