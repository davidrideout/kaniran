//! Port of `ichiran/dict:get-text` (gf — `dict.lisp:18-20`).
//!
//! Returns the most popular text representation (kanji or kana) for a
//! word; the default method delegates to [`super::text::text`].

use std::borrow::Cow;

use super::kani_word::KaniWordDispatchEnum;
use super::text::text;

pub fn get_text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    text(obj)
}
