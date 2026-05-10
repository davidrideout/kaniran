//! Rust-only sidecar — single typed shape for the heterogeneous
//! reading lists that flow through `get-normal-readings`,
//! `make-rmap`, and `match-readings*`.
//!
//! Upstream represents a kanji reading variant as a 2- to 4-element
//! list `(reading type [tag] [gem])`:
//!
//! - 2-elt — `(reading type)` from the ヶ/〆 literals in
//!   [`super::make_rmap`] (e.g. `("か" "ja_on")`, `("が" "abbr")`)
//!   and from the irr fallback in [`super::match_readings_star_`]
//!   (e.g. `("くえ" "irr")`).
//! - 3-elt — `(reading type tag)` from the 〆 literal
//!   `("じめ" "ja_kun" :rendaku)` and as the base entries of
//!   [`super::get_reading_alternatives`] when the explicit `tag` is
//!   `nil`.
//! - 4-elt — `(reading type tag gem)` from the geminate / rendaku
//!   variants of [`super::get_reading_alternatives`].
//!
//! The Rust port crystallises this into one struct with `tag` and
//! `gem` both optional. `tag = None` flags a 2-elt entry (no
//! tag-or-gem positions); `tag = Some(_)` covers the explicit-NIL
//! and explicit-`:rendaku` cases the 3-/4-elt forms encode.
//!
//! Conversion from [`super::get_reading_alternatives::ReadingAlternative`]
//! is provided so the [`super::get_normal_readings`] port can flow
//! its existing typed output into this wider shape without
//! re-querying.

use super::get_reading_alternatives::{ReadingAlternative, ReadingTag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanjiReading {
    pub reading: String,
    pub r#type: String,
    pub tag: Option<ReadingTag>,
    pub gem: Option<String>,
}

impl From<ReadingAlternative> for KanjiReading {
    fn from(a: ReadingAlternative) -> Self {
        Self {
            reading: a.reading,
            r#type: a.r#type,
            tag: Some(a.tag),
            gem: a.gem,
        }
    }
}
