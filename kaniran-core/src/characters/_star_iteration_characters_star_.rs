//! Port of `ichiran/characters:*iteration-characters*`
//! (`characters.lisp:5`).
//!
//! The two iteration marks: plain (`ゝヽ`) and voiced (`ゞヾ`).

use super::kani_kana_class::KanaClass;

pub static ITERATION_CHARACTERS: &[(KanaClass, &str)] = &[
    (KanaClass::Iter, "ゝヽ"),
    (KanaClass::IterV, "ゞヾ"),
];
