//! Port of `ichiran/characters:*dakuten-join*`
//! (`characters.lisp:103`).
//!
//! Pairs of `(input-with-combining-mark, single-precomposed-char)` for
//! both dakuten (゛) and handakuten (゜). Used to normalize the
//! decomposed forms (e.g. `"か゛"` → `"が"`) into single code points.
//!
//! Upstream constructs this by calling the `dakuten-join` function on
//! `*dakuten-hash*` and `*handakuten-hash*`. Until that function is
//! ported the introspected literal value is mirrored here directly,
//! matching the approach used for `*abnormal-chars*` and
//! `*basic-split-regex*`.

pub static DAKUTEN_JOIN: &[(&str, &str)] = &[
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
