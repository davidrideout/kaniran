//! Port of `ichiran/kanji:get-readings-cache` (`kanji.lisp:201`).
//!
//! Returns the readings of `text` whose `type` is not in `typeset`,
//! caching the result in [`KaniranContext::reading_cache`] keyed by
//! `(text, typeset)`. On cache miss, joins `kanji` and `reading` on
//! `kanji.id` and filters by `kanji.text = text` and
//! `reading.type NOT IN typeset`.
//!
//! Empty `typeset` returns no rows: upstream's
//! `(:not (:in 'r.type (:set typeset)))` expands to
//! `r.type NOT IN (NULL)` which is UNKNOWN for every row. The Rust
//! port short-circuits to an empty `Vec` and still records the
//! `(text, [])` → `[]` entry so subsequent calls hit the cache.
//!
//! Diverges from the upstream lambda list `(str typeset)` only by
//! taking `&KaniranContext` for the database handle, replacing the
//! upstream dynamic `*connection*` per [`crate::conn::kani_context`].

use crate::conn::kani_context::KaniranContext;

pub async fn get_readings_cache(
    ctx: &KaniranContext,
    text: &str,
    typeset: &[String],
) -> Result<Vec<(String, String)>, sqlx::Error> {
    let key = (text.to_string(), typeset.to_vec());
    {
        let cache = ctx.reading_cache.lock().unwrap();
        if let Some(val) = cache.get(&key) {
            return Ok(val.clone());
        }
    }
    let result: Vec<(String, String)> = if typeset.is_empty() {
        Vec::new()
    } else {
        // kanji.lisp:206 ((:select 'r.text 'r.type :from (:as 'kanji 'k) ...))
        sqlx::query_as::<_, (String, String)>(
            "SELECT r.text, r.type FROM kanji k \
             INNER JOIN reading r ON r.kanji_id = k.id \
             WHERE k.text = $1 AND r.type <> ALL($2)",
        )
        .bind(text)
        .bind(typeset)
        .fetch_all(&ctx.pool)
        .await?
    };
    {
        let mut cache = ctx.reading_cache.lock().unwrap();
        cache.insert(key, result.clone());
    }
    Ok(result)
}
