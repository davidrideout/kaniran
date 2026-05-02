//! Port of `ichiran/characters:*half-width-kana*`
//! (`characters.lisp:106`).
//!
//! 59-character string of half-width katakana glyphs, paired
//! index-by-index with [`FULL_WIDTH_KANA`][super::_star_full_width_kana_star_::FULL_WIDTH_KANA].
//! Also forms the kana suffix of `*abnormal-chars*`.

pub static HALF_WIDTH_KANA: &str =
    "･ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝﾞﾟ";
