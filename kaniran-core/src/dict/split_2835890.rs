//! Port of `ichiran/dict:split-2835890` (`dict-split.lisp:519`).
//!
//! Registered in [`crate::dict::_star_split_map_star_`] for seq `2835890`.
//! Generated upstream by `def-simple-split` (`dict-split.lisp:519`).
//!
//! Diverges from the upstream lambda list `(reading)` by taking
//! `&KaniranContext` for the database handle, replacing Lisp's dynamic
//! `*connection*` per [`crate::conn::kani_context`].

use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_seq::find_word_seq;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

pub async fn split_2835890(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
) -> Result<(Vec<Option<SplitPart>>, i32), sqlx::Error> {
    let txt: String = reading.true_text().to_string();
    let r = reading;
    let mut offset: usize = 0;
    let mut parts: Vec<Option<SplitPart>> = Vec::new();
    let score: i32 = 50;

    {
        let pseq: &[i32] = &[1385860i32];
        let part_length: Option<usize> = Some(5usize);
        let part_txt = crate::characters::safe_subseq::safe_subseq(&txt, offset, part_length.map(|pl| offset + pl));
        let pushed: Option<SplitPart> = if pseq.contains(&2835890i32) {
            None
        } else if let Some(pt) = part_txt {
            let pt_modified: String = pt.clone();
            find_word_seq(ctx, &pt_modified, pseq).await?.first_word().map(SplitPart::Word)
        } else {
            None
        };
        parts.push(pushed);
        if let Some(pl) = part_length {
            offset += pl;
        }
    }

    {
        let pseq: &[i32] = &[1319060i32];
        let part_length: Option<usize> = Some(1usize);
        let part_txt = crate::characters::safe_subseq::safe_subseq(&txt, offset, part_length.map(|pl| offset + pl));
        let pushed: Option<SplitPart> = if pseq.contains(&2835890i32) {
            None
        } else if let Some(pt) = part_txt {
            let pt_modified: String = pt.clone();
            find_word_seq(ctx, &pt_modified, pseq).await?.first_word().map(SplitPart::Word)
        } else {
            None
        };
        parts.push(pushed);
        if let Some(pl) = part_length {
            offset += pl;
        }
    }

    let _ = (offset, r, &txt, ctx);
    Ok((parts, score))
}
