//! Port of `ichiran/kanji:get-readings-cache` (`kanji.lisp:201`).

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
        // ORDER BY r.id diverges from upstream's unordered SELECT: it returns
        // each kanji's readings in load_readings insertion order (= kanjidic2
        // order), so get_normal_readings' first-occurrence dedup breaks
        // ambiguous-gemination ties deterministically. Without it the JOIN
        // order is unstable and reading.stat_common drifts run-to-run.
        sqlx::query_as::<_, (String, String)>(
            "SELECT r.text, r.type FROM kanji k \
             INNER JOIN reading r ON r.kanji_id = k.id \
             WHERE k.text = $1 AND r.type <> ALL($2) \
             ORDER BY r.id",  // order by is added and not in the original
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
