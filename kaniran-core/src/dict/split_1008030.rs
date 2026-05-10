//! Port of `ichiran/dict:split-1008030` (`dict-split.lisp:588`).
//!
//! Registered in [`crate::dict::_star_split_map_star_`] for seq `1008030`.
//! Generated upstream by `def-simple-split` (`dict-split.lisp:588`).
//!
//! Diverges from the upstream lambda list `(reading)` by taking
//! `&KaniranContext` for the database handle, replacing Lisp's dynamic
//! `*connection*` per [`crate::conn::kani_context`].

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

pub async fn split_1008030(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
) -> Result<(Vec<Option<SplitPart>>, i32), sqlx::Error> {
    let txt: String = reading.true_text().to_string();
    let r = reading;
    let offset: usize = 0;
    let mut parts: Vec<Option<SplitPart>> = Vec::new();
    let score: i32 = -10;

    parts.push(Some(SplitPart::Score));

    let _ = (offset, r, &txt, ctx);
    Ok((parts, score))
}
