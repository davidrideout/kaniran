//! Port of `ichiran/dict:get-text` (gf — `dict.lisp:18-20`).
//!
//! Returns the most popular text representation (kanji or kana) for a
//! word. Three method bodies upstream — the default `(:method (obj)
//! (text obj))` baked into the defgeneric, plus two specializations:
//!
//! - **default `(:method (obj) (text obj))`** at line 20 — for the
//!   word-shaped union ([`KaniWordDispatchEnum`]) the default delegates
//!   to [`super::counters::dispatchers::text`]. Ported as the [`get_text`] free fn
//!   below; it routes through the existing `text` dispatcher (which
//!   handles the counter-text concatenation override at
//!   `dict-counters.lisp:58-59`).
//! - **`((obj entry))`** at line 47-49 — ported on
//!   [`Entry::get_text`] (in [`super::entry_dao`]); selects the
//!   `kanji_text` / `kana_text` row at `ord = 0` based on `n_kanji`.
//!   Reached only from locally-Entry-typed callsites — upstream's
//!   `entry-digest` (`dict.lisp:67`) is the canonical one — so it
//!   stays an inherent method and is not wired through this
//!   dispatcher. No upstream callsite passes an entry polymorphically.
//! - **`((segment segment))`** at line 677-679 — ported on
//!   [`Segment::get_text`] (in [`super::segment_struct`]); lazy
//!   memoization of `text(segment.word)` into the
//!   [`Segment::text`] cache slot.
//!
//! [`KaniWordDispatchEnum`]: super::kani::KaniWordDispatchEnum
//! [`Entry::get_text`]: super::entry_dao::Entry::get_text
//! [`Segment::get_text`]: super::segment_struct::Segment::get_text
//! [`Segment::text`]: super::segment_struct::Segment#structfield.text

use std::borrow::Cow;

use super::kani::KaniWordDispatchEnum;
use super::counters::dispatchers::text;

pub fn get_text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    text(obj)
}
