//! Port of `ichiran/kanji:*kanjidic-path*` (`kanji.lisp:3`).
//!
//! Filesystem path to the kanjidic2.xml dump consumed by
//! `load-kanjidic` (the upstream per-deployment placeholder).

pub static KANJIDIC_PATH: &str = "e:/dump/kanjidic2.xml";
