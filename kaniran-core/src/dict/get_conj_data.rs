//! Transliteration of `ichiran/dict:get-conj-data` (`dict.lisp:340`).
//!
//! Walks the `conjugation` table for one `seq`, joins each row to its
//! `conj_prop` rows and `conj_source_reading` rows, and packs the
//! result as a [`ConjData`] per `(conjugation, conj-prop)` pair. When
//! `texts` is non-empty, conjugations whose `conj_source_reading.text`
//! doesn't intersect that set are dropped.

use crate::conn::kani_context::KaniranContext;
use sqlx::Row;

use super::conj_prop_dao::ConjProp;
use super::conj_data_struct::ConjData;
use super::conjugation_dao::Conjugation;
use super::make_conj_data::make_conj_data;
use super::no_conj_data::no_conj_data;

#[derive(Debug, Clone)]
pub enum FromOrConjIds {
    /// Lisp `NIL` — fetch every conjugation row for the seq.
    All,
    /// Lisp `:root` — short-circuit to an empty result.
    Root,
    /// Lisp integer `from/conj-ids` — fetch conjugations whose
    /// `conjugation.from` column equals the integer.
    From(i32),
    /// Lisp list `from/conj-ids` — fetch conjugations whose `id` is in
    /// the list.
    ConjIds(Vec<i32>),
}

pub async fn get_conj_data(
    ctx: &KaniranContext,
    seq: i32,
    from_or_conj_ids: FromOrConjIds,
    texts: &[&str],
) -> Result<Vec<ConjData>, sqlx::Error> {
    if matches!(from_or_conj_ids, FromOrConjIds::Root) || no_conj_data(ctx, seq) {
        return Ok(Vec::new());
    }

    let filter_by_texts = !texts.is_empty();
    let texts_owned: Vec<String> = texts.iter().map(|s| (*s).to_string()).collect();

    let conjs: Vec<Conjugation> = match &from_or_conj_ids {
        FromOrConjIds::All => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1")
                .bind(seq)
                .fetch_all(&ctx.pool)
                .await?
        }
        FromOrConjIds::ConjIds(ids) => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND id = ANY($2)")
                .bind(seq)
                .bind(ids)
                .fetch_all(&ctx.pool)
                .await?
        }
        FromOrConjIds::From(from) => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND \"from\" = $2")
                .bind(seq)
                .bind(*from)
                .fetch_all(&ctx.pool)
                .await?
        }
        FromOrConjIds::Root => unreachable!("filtered out at the top of the function"),
    };

    let mut out: Vec<ConjData> = Vec::new();
    for conj in conjs {
        let src_rows = if filter_by_texts {
            sqlx::query(
                "SELECT text, source_text FROM conj_source_reading \
                 WHERE conj_id = $1 AND text = ANY($2)",
            )
            .bind(conj.id)
            .bind(&texts_owned)
            .fetch_all(&ctx.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT text, source_text FROM conj_source_reading WHERE conj_id = $1",
            )
            .bind(conj.id)
            .fetch_all(&ctx.pool)
            .await?
        };
        let src_map: Vec<(String, String)> = src_rows
            .into_iter()
            .map(|row| -> Result<(String, String), sqlx::Error> {
                Ok((row.try_get("text")?, row.try_get("source_text")?))
            })
            .collect::<Result<_, _>>()?;
        if filter_by_texts && src_map.is_empty() {
            continue;
        }

        let props: Vec<ConjProp> = sqlx::query_as("SELECT * FROM conj_prop WHERE conj_id = $1")
            .bind(conj.id)
            .fetch_all(&ctx.pool)
            .await?;
        for prop in props {
            out.push(make_conj_data(
                Some(conj.seq),
                Some(conj.seq_from),
                conj.seq_via,
                Some(prop),
                src_map.clone(),
            ));
        }
    }
    Ok(out)
}
