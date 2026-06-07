//! Kaniran sidecar (no Lisp FQN). Lite mirror of
//! [`super::path::TopArray`] +
//! [`super::accessors::register_item`] +
//! [`super::accessors::get_array`] used inside the `find-best-path`
//! inner loop, over the lite [`KaniLiteTopArrayItem`] type.

use super::kani_lite_top_array_item::{KaniLitePathElement, KaniLiteTopArrayItem};

#[derive(Debug, Clone)]
pub struct KaniLiteTopArray {
    pub array: Vec<Option<KaniLiteTopArrayItem>>,
    pub count: usize,
}

impl KaniLiteTopArray {
    pub fn new(limit: usize) -> Self {
        Self {
            array: vec![None; limit],
            count: 0,
        }
    }
}

/// Mirror of `register-item` (`dict.lisp:1148`) for the lite item
/// type — same insertion-sort by score with bounded array.
pub fn kani_lite_register_item(
    obj: &mut KaniLiteTopArray,
    score: i32,
    payload: std::sync::Arc<[KaniLitePathElement]>,
) {
    let mut item: Option<KaniLiteTopArrayItem> = Some(KaniLiteTopArrayItem { score, payload });
    let len = obj.array.len();
    let start = obj.count.min(len);
    let mut idx = start;
    loop {
        let prev_score = if idx > 0 {
            obj.array[idx - 1].as_ref().map(|prev| prev.score)
        } else {
            None
        };
        let done = match prev_score {
            None => true,
            Some(prev) => prev >= score,
        };
        if idx < len {
            obj.array[idx] = if done {
                item.take()
            } else {
                obj.array[idx - 1].take()
            };
        }
        if done {
            break;
        }
        idx -= 1;
    }
    obj.count += 1;
}

/// Mirror of `get-array` (`dict.lisp:1145`) for the lite array — the
/// first `count` slots filled by insertion.
pub fn kani_lite_get_array(top: &KaniLiteTopArray) -> &[Option<KaniLiteTopArrayItem>] {
    let used = top.count.min(top.array.len());
    &top.array[..used]
}
