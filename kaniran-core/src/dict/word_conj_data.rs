//! Port of `ichiran/dict:word-conj-data` (`dict.lisp:654`).
//!
//! Returns the conjugation data for a word — the [`Vec<ConjData>`]
//! produced by [`super::get_conj_data::get_conj_data`] for readings
//! whose `word-conjugations` slot directs the lookup. Three method
//! arms in the Lisp gf:
//!
//! - **simple-text** (`dict.lisp:657-658`) — `(get-conj-data (seq w)
//!   (word-conjugations w) (true-text w))`. The
//!   [`super::simple_text_class::WordConjugations`] slot value picks
//!   the [`super::get_conj_data::FromOrConjIds`] arm: `nil` → `All`,
//!   `:root` → `Root`, integer-list → `ConjIds`. The `texts` filter is
//!   the reading's surface form (single string).
//! - **compound-text** (`dict.lisp:660-661`) — recurse on the last
//!   word in `words`. Compounds are built from simple-texts only via
//!   `adjoin-word`, but the dispatch is faithful and would handle a
//!   nested compound through the recursive call.
//! - **counter-text** (`dict-counters.lisp:87`) — always `nil`.
//!
//! Diverges from the upstream lambda list `(word)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`]. The proxy
//! arm of the simple-text method's `(seq w)` / `(word-conjugations w)`
//! / `(true-text w)` triple is inlined here as a single proxy-chain
//! walk — the leaf is a kanji-text or kana-text and all three slot
//! reads come from it. (The proxy-text gf overrides for `seq`,
//! `word-conjugations`, and `true-text` all delegate to `(source obj)`
//! without touching the proxy's own `text` / `state`, so the leaf walk
//! is equivalent.)

use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data_struct::ConjData;
use crate::dict::get_conj_data::{get_conj_data, FromOrConjIds};
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::simple_text_class::WordConjugations;

pub async fn word_conj_data(
    ctx: &KaniranContext,
    word: &KaniWordDispatchEnum,
) -> Result<Vec<ConjData>, sqlx::Error> {
    match word {
        // dict-counters.lisp:87 (defmethod word-conj-data ((obj counter-text)) nil)
        KaniWordDispatchEnum::Counter(_) => Ok(Vec::new()),

        // dict.lisp:660-661 (defmethod word-conj-data ((word compound-text)))
        KaniWordDispatchEnum::Compound(c) => {
            let last = c
                .words
                .last()
                .expect("compound-text always has at least one word (adjoin-word ctor)");
            Box::pin(word_conj_data(ctx, last)).await
        }

        // dict.lisp:657-658 (defmethod word-conj-data ((word simple-text)))
        KaniWordDispatchEnum::Kanji(k) => {
            simple_text_arm(ctx, k.seq, &k.state.conjugations, &k.text).await
        }
        KaniWordDispatchEnum::Kana(k) => {
            simple_text_arm(ctx, k.seq, &k.state.conjugations, &k.text).await
        }
        KaniWordDispatchEnum::Proxy(p) => {
            // dict.lisp:657-658 — proxy's seq / word-conjugations / true-text gfs
            // each delegate to (source obj); the leaf is a kanji-text or kana-text
            // and supplies all three slot reads.
            let leaf = leaf_of(p);
            let (seq, conj, text) = leaf_slots(leaf);
            simple_text_arm(ctx, seq, conj, text).await
        }
    }
}

async fn simple_text_arm(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: &Option<WordConjugations>,
    true_text: &str,
) -> Result<Vec<ConjData>, sqlx::Error> {
    let from_or_conj_ids = match conjugations {
        None => FromOrConjIds::All,
        Some(WordConjugations::Root) => FromOrConjIds::Root,
        Some(WordConjugations::Ids(ids)) => FromOrConjIds::ConjIds(ids.clone()),
    };
    get_conj_data(ctx, seq, from_or_conj_ids, &[true_text]).await
}

fn leaf_of(p: &ProxyText) -> &KaniSimpleTextDispatchEnum {
    let mut current: &KaniSimpleTextDispatchEnum = &p.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Proxy(inner) => current = &inner.source,
            _ => return current,
        }
    }
}

fn leaf_slots(
    leaf: &KaniSimpleTextDispatchEnum,
) -> (i32, &Option<WordConjugations>, &str) {
    match leaf {
        KaniSimpleTextDispatchEnum::Kanji(k) => (k.seq, &k.state.conjugations, &k.text),
        KaniSimpleTextDispatchEnum::Kana(k) => (k.seq, &k.state.conjugations, &k.text),
        KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!("leaf_of strips proxies"),
    }
}
