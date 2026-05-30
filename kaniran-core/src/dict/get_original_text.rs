//! Port of `ichiran/dict:get-original-text` (gf — `dict.lisp:394`).
//!
//! Returns the unconjugated reading rows for a `reading`. Two upstream
//! methods:
//! - simple-text (kanji-text / kana-text, `dict.lisp:396-400`): resolves
//!   `(text reading)` through [`get_original_text_star_`] using `conj-data`
//!   (or, when the keyword is absent, the simple-text body of
//!   `word-conj-data` applied to `reading`), then looks up each
//!   `(txt, seq)` pair in either the `kanji_text` or `kana_text` table
//!   per `(word-type reading)`.
//! - proxy-text (`dict.lisp:589-590`): recurses on `(source reading)`.
//!
//! Diverges from the upstream lambda list `(reading &key conj-data)` by:
//! - taking `&KaniranContext` for the database handle (replacing
//!   Lisp's `*connection*`) per [`crate::conn::kani_context`];
//! - replacing the `&key conj-data` keyword with `Option<&[ConjData]>`
//!   (`None` ≡ keyword absent ≡ Lisp `NIL`);
//! - returning `Vec<KaniSimpleTextDispatchEnum>` (Kanji or Kana
//!   variants only — see below) so callers can pull both
//!   `kanji-text` rows (when `word-type = :kanji`) and `kana-text`
//!   rows (when `:kana`) from the same dispatcher.
//!
//! Polymorphic surface: only the simple-text union
//! ([`KaniSimpleTextDispatchEnum`]) reaches this generic upstream —
//! the two methods are defined on `simple-text` and `proxy-text`.
//! Callers holding the wider [`crate::dict::kani::KaniWordDispatchEnum`]
//! don't invoke `get-original-text` (no methods for compound-text or
//! counter-text).

use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data_struct::ConjData;
use crate::dict::get_conj_data::{get_conj_data, FromOrConjIds};
use crate::dict::get_original_text_star_::get_original_text_star_;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniSimpleTextDispatchEnum;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::simple_text_class::WordConjugations;
use crate::dict::word_type::WordType;

pub async fn get_original_text(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_data: Option<&[ConjData]>,
) -> Result<Vec<KaniSimpleTextDispatchEnum>, sqlx::Error> {
    match reading {
        // dict.lisp:589-590 (defmethod get-original-text ((reading proxy-text)))
        KaniSimpleTextDispatchEnum::Proxy(p) => {
            Box::pin(get_original_text(ctx, &p.source, conj_data)).await
        }
        // dict.lisp:396-400 (defmethod get-original-text ((reading simple-text)))
        KaniSimpleTextDispatchEnum::Kanji(_) | KaniSimpleTextDispatchEnum::Kana(_) => {
            simple_text_arm(ctx, reading, conj_data).await
        }
    }
}

async fn simple_text_arm(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_data: Option<&[ConjData]>,
) -> Result<Vec<KaniSimpleTextDispatchEnum>, sqlx::Error> {
    let (seq_value, conjugations, reading_text, word_type) = match reading {
        KaniSimpleTextDispatchEnum::Kanji(k) => {
            (k.seq, &k.state.conjugations, k.text.as_str(), WordType::Kanji)
        }
        KaniSimpleTextDispatchEnum::Kana(k) => {
            (k.seq, &k.state.conjugations, k.text.as_str(), WordType::Kana)
        }
        KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!("dispatched above"),
    };

    let owned_cd: Vec<ConjData>;
    let cd: &[ConjData] = match conj_data {
        Some(cd) => cd,
        None => {
            // dict.lisp:657-658 (defmethod word-conj-data ((word simple-text)))
            // — inlined because `reading` is statically simple-text here
            // (proxy-text was peeled in the dispatcher above), so the
            // simple-text method body applies directly without wrapping
            // the reading in [`KaniWordDispatchEnum`] just to dispatch.
            let from_or_conj_ids = match conjugations {
                None => FromOrConjIds::All,
                Some(WordConjugations::Root) => FromOrConjIds::Root,
                Some(WordConjugations::Ids(ids)) => FromOrConjIds::ConjIds(ids.clone()),
            };
            owned_cd =
                get_conj_data(ctx, seq_value, from_or_conj_ids, &[reading_text]).await?;
            &owned_cd
        }
    };

    let orig_texts = get_original_text_star_(ctx, cd, &[reading_text]).await?;

    let mut rows: Vec<KaniSimpleTextDispatchEnum> = Vec::new();
    for (txt, seq_n) in orig_texts {
        // dict.lisp:399-400 ((select-dao table (:and (:= 'seq seq) (:= 'text txt))))
        match word_type {
            WordType::Kanji => {
                let fetched: Vec<KanjiText> = sqlx::query_as(
                    "SELECT * FROM kanji_text WHERE seq = $1 AND text = $2",
                )
                .bind(seq_n)
                .bind(&txt)
                .fetch_all(&ctx.pool)
                .await?;
                for row in fetched {
                    rows.push(KaniSimpleTextDispatchEnum::Kanji(row));
                }
            }
            WordType::Kana => {
                let fetched: Vec<KanaText> = sqlx::query_as(
                    "SELECT * FROM kana_text WHERE seq = $1 AND text = $2",
                )
                .bind(seq_n)
                .bind(&txt)
                .fetch_all(&ctx.pool)
                .await?;
                for row in fetched {
                    rows.push(KaniSimpleTextDispatchEnum::Kana(row));
                }
            }
            WordType::Gap => unreachable!(
                "simple-text variants always have word-type :kanji or :kana"
            ),
        }
    }
    Ok(rows)
}
