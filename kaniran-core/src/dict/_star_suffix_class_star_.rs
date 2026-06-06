//! Port of `ichiran/dict:*suffix-class*` (`dict-grammar.lisp:6`).
//!
//! JMdict seq → suffix class (`:teiru`, `:te`, `:iru`, `:ha`, …) the
//! entry belongs to.

use crate::conn::kani_context::KaniranContext;
use std::collections::HashMap;

pub type SuffixClass = HashMap<i32, String>;

pub fn suffix_class(ctx: &KaniranContext) -> &SuffixClass {
    &ctx.suffix_class
}
