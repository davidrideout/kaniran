//! Port of `ichiran/characters:*punctuation-marks*`
//! (`characters.lisp:85-91`).
//!
//! Pairs of `(japanese-mark, ascii-equivalent)` used to romanize
//! punctuation. 18 entries.

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
