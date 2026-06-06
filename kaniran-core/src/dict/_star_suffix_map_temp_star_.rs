//! Port of `ichiran/dict:*suffix-map-temp*` (`dict.lisp:1049`).
//!
//! Caller-scoped suffix lookup cache: character end-position →
//! `(substr, keyword, kf)` suffix candidates ending there, letting
//! `find-word-suffix` skip recomputing them via `get-suffixes`.

use crate::dict::kana_text_dao::KanaText;
use std::collections::HashMap;

pub type SuffixMapTemp = HashMap<usize, Vec<(String, String, Option<KanaText>)>>;
