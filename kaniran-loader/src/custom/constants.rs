/// Port of `ichiran/custom:*municipality-types*` (`dict-custom.lisp:97`).
pub static MUNICIPALITY_TYPES: &[(char, &[&str])] = &[
    ('都', &["ﾄ", "と"]),
    ('道', &["ﾄﾞｳ", "どう"]),
    ('府', &["ﾌ", "ふ"]),
    ('県', &["ｹﾝ", "けん"]),
    ('市', &["ｼ", "し"]),
    ('町', &["ﾁｮｳ", "ﾏﾁ", "ちょう", "まち"]),
    ('村', &["ｿﾝ", "ﾑﾗ", "そん", "むら"]),
    ('区', &["ｸ", "く"]),
];

/// Port of `ichiran/custom:*municipality-types-description*` (`dict-custom.lisp:107`).
///
/// 道's upstream entry is the bare cons `(#\道)` with `nil` cdr,
/// ported as `None`.
pub static MUNICIPALITY_TYPES_DESCRIPTION: &[(char, Option<&str>)] = &[
    ('都', Some("Metropolis")),
    ('道', None),
    ('府', Some("Prefecture")),
    ('県', Some("Prefecture")),
    ('市', Some("(city)")),
    ('町', Some("(town)")),
    ('村', Some("(village)")),
    ('区', Some("Ward")),
];

/// Port of `ichiran/custom:*municipality-types-order*` (`dict-custom.lisp:118`).
pub static MUNICIPALITY_TYPES_ORDER: &str = "都道府県市区町村";
