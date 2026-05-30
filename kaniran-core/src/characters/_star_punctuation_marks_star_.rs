//! Port of `ichiran/characters:*punctuation-marks*`
//! (`characters.lisp:85-91`).
//!
//! Pairs of `(japanese-mark, ascii-equivalent)` used to romanize
//! punctuation. 18 entries. Same shape as the value returned by
//! [`super::voicing_tables::dakuten_join`].
//!
//! Note: the introspected md uses Lisp's shared-structure reader
//! notation (`#1=` / `#1#`, `#2=` / `#2#`) for two distinct repeated
//! string values — `", "` (entries 3 and 4) and a single ASCII space
//! (entries 7 and 8). Semantically those are plain repeated strings,
//! not references; the notation is just `prin1`'s way of showing that
//! the reader had shared one string-object across both positions.

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
