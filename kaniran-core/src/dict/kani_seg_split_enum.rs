//! Kaniran sidecar (no Lisp FQN). Fixed-shape split candidate emitted
//! by `get-penalties` / `get-synergies` / `get-seg-splits` — upstream
//! returns the list `(seg-right [synergy] seg-left)`; carrying the two
//! Arcs inline keeps the dominant no-penalty case off the heap.

use std::sync::Arc;

use super::grammar::synergy::Synergy;
use super::kani_lite_segment_list::KaniLiteSegmentList;

/// One `(seg-right [synergy] seg-left)` split candidate between two
/// adjacent segment-lists.
///
/// `Plain` — no penalty or synergy fired (the common case): for
/// 「体に」+「よくない」, `Plain { right: よくない-list, left: 体に-list }`.
/// `WithSynergy` — a penalty/synergy sits between the pair: two short
/// kana words like 「あ」+「で」 carry `penalty-short`, giving
/// `WithSynergy { right: で-list, synergy: Synergy { description:
/// "short", score: -9, .. }, left: あ-list }`.
#[derive(Debug, Clone)]
pub enum KaniSegSplitEnum {
    Plain {
        right: Arc<KaniLiteSegmentList>,
        left: Arc<KaniLiteSegmentList>,
    },
    WithSynergy {
        right: Arc<KaniLiteSegmentList>,
        synergy: Synergy,
        left: Arc<KaniLiteSegmentList>,
    },
}
