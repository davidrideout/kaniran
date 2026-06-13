use crate::conn::kani_backend::KaniBackend;
use crate::characters::char_class::{test_word, CharClass};
use std::borrow::Cow;
use crate::characters::kanji::{kanji_cross_match, kanji_match, kanji_regex};
use crate::conn::kani_context::KaniranContext;
use crate::core::methods::{default_romanization_method, RomanizationMethod};
use crate::core::romanize::romanize_word;
use crate::dict::conj::{get_conj_data, ConjData, FromOrConjIds};
use crate::dict::dao::{Entry, KanaText, KanjiText, WordConjugations};
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::path::{SubstringHash, MAX_WORD_LENGTH};

/// Port of `ichiran/dict:get-kanji-kana-old` (`dict.lisp:117-124`).
///
/// Fallback `get-kana` body for the kanji-text method. Builds a regex
/// from the kanji-text's surface form, walks the entry's `kana_text`
/// rows in `ord` order and returns the first kana whose text matches;
/// if none match, returns the first kana row's text.
pub async fn get_kanji_kana_old(
    ctx: &KaniranContext,
    obj: &KanjiText,
) -> Result<Option<String>, sqlx::Error> {
    let regex = kanji_regex(&obj.text);
    let kts = ctx.store.kana_texts_by_seq_ordered(obj.seq).await?;
    for kt in &kts {
        if regex.is_match(&kt.text).unwrap_or(false) {
            return Ok(Some(kt.text.clone()));
        }
    }
    Ok(kts.into_iter().next().map(|kt| kt.text))
}

/// Transliteration of `ichiran/dict:get-original-text-once` (`dict.lisp:371`).
///
/// For each conj-data, collects the source-text of every `src-map` pair
/// whose text is in `texts`, concatenated across all conj-datas in order.
pub fn get_original_text_once(conj_datas: &[ConjData], texts: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for conj_data in conj_datas {
        for (txt, src_txt) in &conj_data.src_map {
            if texts.contains(&txt.as_str()) {
                out.push(src_txt.clone());
            }
        }
    }
    out
}

/// Port of `ichiran/dict:get-original-text*` (`dict.lisp:380`).
///
/// For each input [`ConjData`] and the surface `texts`, returns
/// `(text, seq)` pairs naming the pre-conjugation reading rows to look
/// up: matched `src_text` paired with the conj-data's `from` when `via`
/// is nil, else recursing through [`get_conj_data`] to peel one layer
/// of secondary conjugation when `via` is set.
pub async fn get_original_text_star_(
    ctx: &KaniranContext,
    conj_datas: &[ConjData],
    texts: &[&str],
) -> Result<Vec<(String, i32)>, sqlx::Error> {
    let mut out: Vec<(String, i32)> = Vec::new();
    for conj_data in conj_datas {
        let src_text: Vec<&str> = conj_data
            .src_map
            .iter()
            .filter(|(txt, _)| texts.iter().any(|t| *t == txt))
            .map(|(_, src_txt)| src_txt.as_str())
            .collect();
        let from = conj_data
            .from
            .expect("conj-data emitted by get-conj-data always has a non-nil `from` slot");
        match conj_data.via {
            None => {
                // dict.lisp:390 ((mapcar (lambda (txt) (list txt (conj-data-from conj-data))) src-text))
                for txt in &src_text {
                    out.push(((*txt).to_string(), from));
                }
            }
            Some(via) => {
                // dict.lisp:391-392 (recursive get-conj-data + get-original-text*)
                let new_cd = get_conj_data(ctx, via, FromOrConjIds::From(from), &[]).await?;
                let inner = Box::pin(get_original_text_star_(ctx, &new_cd, &src_text)).await?;
                out.extend(inner);
            }
        }
    }
    Ok(out)
}

/// Port of `ichiran/dict:query-parents-kanji` (`dict.lisp:404`).
///
/// For a given `seq` and surface `text`, enumerates the
/// `(kanji-text.id, conjugation.id)` pairs whose `kanji-text` row is the
/// parent reading of the `conj-source-reading` that produced
/// `(seq, text)` — the source `seq` resolves to the conjugation's `via`
/// when set, otherwise its `from`.
pub async fn query_parents_kanji(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    ctx.store.parents_kanji(seq, text).await
}

/// Port of `ichiran/dict:query-parents-kana` (`dict.lisp:417`).
///
/// For a given `seq` and surface `text`, enumerates the
/// `(kana-text.id, conjugation.id)` pairs whose `kana-text` row is the
/// parent reading of the `conj-source-reading` that produced
/// `(seq, text)` — the source `seq` resolves to the conjugation's `via`
/// when set, otherwise its `from`.
pub async fn query_parents_kana(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    ctx.store.parents_kana(seq, text).await
}

/// Port of `ichiran/dict:best-kana-conj` (`dict.lisp:430`).
///
/// For a [`KanjiText`] reading row, return the kana surface form to
/// display, walking the conjugation chain back through parent readings
/// when the pre-baked `best_kana` slot doesn't apply.
pub async fn best_kana_conj(
    ctx: &KaniranContext,
    obj: &KanjiText,
) -> Result<Option<String>, sqlx::Error> {
    let wc = &obj.state.conjugations;
    // dict.lisp:431-433 ((and (or (not wc) (eql wc :root)) (not (eql (best-kana obj) :null))))
    let root_or_unset = matches!(wc, None | Some(WordConjugations::Root));
    if root_or_unset && obj.best_kana.is_some() {
        return Ok(obj.best_kana.clone());
    }

    let parents = query_parents_kanji(ctx, obj.seq, &obj.text).await?;
    for (pid, cid) in parents {
        // dict.lisp:436 (for parent-kt = (get-dao 'kanji-text pid))
        // The store method's fetch_one mirrors upstream: a missing pid
        // would surface as nil from get-dao and crash on the next slot
        // access; propagating the sqlx error preserves that fail-loud
        // stance.
        let parent_kt: KanjiText = ctx.store.kanji_text_by_id(pid).await?;
        // dict.lisp:437 (for parent-bk = (best-kana-conj parent-kt))
        let parent_bk = Box::pin(best_kana_conj(ctx, &parent_kt)).await?;
        // dict.lisp:438 (unless (or (eql parent-bk :null)
        //                           (and wc (or (eql wc :root) (not (find cid wc))))))
        let skip = parent_bk.is_none()
            || match wc {
                None => false,
                Some(WordConjugations::Root) => true,
                Some(WordConjugations::Ids(ids)) => !ids.contains(&cid),
            };
        if skip {
            continue;
        }
        let parent_bk = parent_bk.expect("checked Some via `skip` above");

        // dict.lisp:439-442 (query (:select 'text :from 'conj-source-reading
        //   :where (:and (:= 'conj-id cid) (:= 'source-text parent-bk))) :column)
        let readings: Vec<String> = ctx.store.conj_source_reading_texts(cid, &parent_bk).await?;
        if readings.is_empty() {
            continue;
        }
        if readings.len() == 1 {
            return Ok(Some(readings.into_iter().next().unwrap()));
        }
        // dict.lisp:447 (km = (kanji-cross-match (text parent-kt) parent-bk (text obj)))
        let km = kanji_cross_match(&parent_kt.text, &parent_bk, &obj.text);
        if let Some(km_text) = &km {
            // dict.lisp:448 (find km readings :test 'equal)
            if let Some(hit) = readings.iter().find(|r| *r == km_text) {
                return Ok(Some(hit.clone()));
            }
        }
        // dict.lisp:449-454 (stable-sort by |len(r) - len-km| then first regex match,
        // falling back to (car readings) — SBCL's destructive stable-sort
        // leaves the `readings` variable pointing at the original head
        // cell, whose car is unchanged, so the fallback returns the
        // pre-sort first reading. Capture it before sorting to preserve
        // that.
        let first_reading = readings[0].clone();
        let regex = kanji_regex(&obj.text);
        let len_km = km.as_ref().map(|s| s.chars().count() as i64).unwrap_or(0);
        let mut sorted = readings;
        sorted.sort_by_key(|r| (r.chars().count() as i64 - len_km).abs());
        for rd in &sorted {
            if regex.is_match(rd).unwrap_or(false) {
                return Ok(Some(rd.clone()));
            }
        }
        return Ok(Some(first_reading));
    }
    Ok(None)
}

/// Port of `ichiran/dict:best-kanji-conj` (`dict.lisp:457`).
///
/// Given a [`KanaText`] reading row, return the kanji surface form to
/// display, walking the conjugation chain back through parent readings
/// when the pre-baked `best_kanji` slot doesn't apply.
pub async fn best_kanji_conj(
    ctx: &KaniranContext,
    obj: &KanaText,
) -> Result<Option<String>, sqlx::Error> {
    let wc = &obj.state.conjugations;
    // dict.lisp:458-460 ((and (or (not wc) (eql wc :root)) (not (eql (best-kanji obj) :null))))
    let root_or_unset = matches!(wc, None | Some(WordConjugations::Root));
    if root_or_unset && obj.best_kanji.is_some() {
        return Ok(obj.best_kanji.clone());
    }

    // dict.lisp:461-462 ((or (nokanji obj) (= (n-kanji (get-dao 'entry (seq obj))) 0)) :null)
    if obj.nokanji {
        return Ok(None);
    }
    // Option → RowNotFound mirrors the original fetch_one: a missing
    // entry row is a data-integrity violation that must fail loud.
    let entry: Entry = ctx
        .store
        .entry_by_seq(obj.seq)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    if entry.n_kanji == 0 {
        return Ok(None);
    }

    let parents = query_parents_kana(ctx, obj.seq, &obj.text).await?;
    for (pid, cid) in parents {
        // dict.lisp:465 (for parent-bk = (best-kanji-conj (get-dao 'kana-text pid)))
        let parent_kt: KanaText = ctx.store.kana_text_by_id(pid).await?;
        let parent_bk = Box::pin(best_kanji_conj(ctx, &parent_kt)).await?;
        // dict.lisp:466 (unless (or (eql parent-bk :null)
        //                           (and wc (or (eql wc :root) (not (find cid wc))))))
        let skip = parent_bk.is_none()
            || match wc {
                None => false,
                Some(WordConjugations::Root) => true,
                Some(WordConjugations::Ids(ids)) => !ids.contains(&cid),
            };
        if skip {
            continue;
        }
        let parent_bk = parent_bk.expect("checked Some via `skip` above");

        // dict.lisp:467-470 (query (:select 'text :from 'conj-source-reading
        //   :where (:and (:= 'conj-id cid) (:= 'source-text parent-bk))) :column)
        let readings: Vec<String> = ctx.store.conj_source_reading_texts(cid, &parent_bk).await?;
        // dict.lisp:471-473 (some (lambda (reading) (and (kanji-match reading (text obj)) reading)) readings)
        if let Some(hit) = readings.into_iter().find(|r| kanji_match(r, &obj.text)) {
            return Ok(Some(hit));
        }
    }
    Ok(None)
}

/// Port of `ichiran/dict:find-word` (`dict.lisp:489`).
///
/// Looks up rows of `kana_text` / `kanji_text` whose `text` column
/// equals `word`. Picks the table by [`test_word`] against
/// [`CharClass::Kana`] — kana inputs hit `kana_text`, anything else
/// hits `kanji_text`. With `root_only=true`, an inner-join against
/// `entry` restricts results to entries flagged `root_p`
/// (derivation-pruning survivors).
///
/// When the ctx's `*substring-hash*` cache holds `word` as a key, the
/// cached rows short-circuit the SQL query (root-only is excluded from
/// the short-circuit).
#[derive(Debug, Clone)]
pub enum FindWordRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

pub async fn find_word<'a>(
    ctx: &'a KaniranContext,
    word: &str,
    root_only: bool,
) -> Result<Cow<'a, FindWordRows>, sqlx::Error> {
    // Mirror upstream evaluation order — `(when (<= (length word)
    // *max-word-length*) ...)` short-circuits before `test-word`
    // runs, so the over-length path returns an empty result without
    // touching the kana/kanji predicate. Lisp returns plain nil; the
    // Rust shape (closed 2-variant per CONVENTIONS §4.3) demands a
    // tag, so we hardcode `Kanji(Vec::new())` — every consumer
    // iterates the variant as a list and observes only the (empty)
    // contents, never the tag, so the choice is arbitrary and a
    // fixed value avoids the spurious `test_word` call.
    if word.chars().count() > MAX_WORD_LENGTH {
        return Ok(Cow::Owned(FindWordRows::Kanji(Vec::new())));
    }
    // dict.lisp:491 — (and *substring-hash* (gethash word *substring-hash*))
    // A hit returns the cached bucket borrowed (the hash lives on ctx
    // for the whole call tree), so the hot path stops deep-cloning
    // every row; only callers that need ownership pay for a copy.
    if !root_only {
        if let Some(cache) = ctx.substring_hash.as_deref() {
            if let Some(rows) = cache.get(word) {
                return Ok(Cow::Borrowed(rows));
            }
        }
    }
    let kana = test_word(word, CharClass::Kana);
    if kana {
        let rows: Vec<KanaText> = if root_only {
            ctx.store.kana_texts_root_by_text(word).await?
        } else {
            ctx.store.kana_texts_by_text(word).await?
        };
        Ok(Cow::Owned(FindWordRows::Kana(rows)))
    } else {
        let rows: Vec<KanjiText> = if root_only {
            ctx.store.kanji_texts_root_by_text(word).await?
        } else {
            ctx.store.kanji_texts_by_text(word).await?
        };
        Ok(Cow::Owned(FindWordRows::Kanji(rows)))
    }
}

/// Port of `ichiran/dict:find-substring-words` (`dict.lisp:501`).
///
/// Builds the substring lookup cache: enumerates every length-bounded
/// substring of `str` whose `(start, end)` pair is not blocked by
/// `sticky` (positions that cannot serve as a word boundary), then
/// bulk-fetches the matching `kana_text` / `kanji_text` rows, bucketed
/// by substring text. Substrings absent from the database still get an
/// empty hash entry, signalling that the lookup already ran.
pub async fn find_substring_words(
    ctx: &KaniranContext,
    str: &str,
    sticky: &[usize],
) -> Result<SubstringHash, sqlx::Error> {
    // dict.lisp:504-512 (loop for start ... loop for end ...). CONVENTIONS
    // §4.5: cl-ppcre / subseq index by character — collect the chars
    // once so the inner slice uses character offsets.
    let chars: Vec<char> = str.chars().collect();
    let n = chars.len();

    // Upper bound on (start, end) pairs — sizes the keys vectors and
    // keeps the hot loop free of growth reallocations.
    let pair_bound = n * MAX_WORD_LENGTH;
    let mut kana_keys: Vec<String> = Vec::with_capacity(pair_bound);
    let mut kanji_keys: Vec<String> = Vec::with_capacity(pair_bound);

    for start in 0..n {
        if sticky.contains(&start) {
            continue;
        }
        // (min (length str) (+ start *max-word-length*))
        let end_max = n.min(start + MAX_WORD_LENGTH);
        for end in (start + 1)..=end_max {
            if sticky.contains(&end) {
                continue;
            }
            // (subseq str start end) — character offsets per §4.5.
            let part: String = chars[start..end].iter().collect();
            if test_word(&part, CharClass::Kana) {
                kana_keys.push(part);
            } else {
                kanji_keys.push(part);
            }
        }
    }

    // dict.lisp:513 — (mapcar 'remove-duplicates (list kana-keys kanji-keys))
    // The upstream remove-duplicates keeps the last occurrence of each
    // string; downstream consumes the list as a SQL IN-set, so order
    // and which occurrence is dropped don't matter at the boundary —
    // sort+dedup is the cheap canonical form for the bulk query.
    kana_keys.sort();
    kana_keys.dedup();
    kanji_keys.sort();
    kanji_keys.dedup();

    // dict.lisp:510 — pre-populate the hash with an empty entry per
    // substring. Done after dedup so the map is built once at exact
    // capacity with one insert per unique key; the variant choice is a
    // pure function of the text, so populating from the deduped keys
    // produces the same map as populating inside the loop above.
    let mut substring_hash: SubstringHash =
        SubstringHash::with_capacity(kana_keys.len() + kanji_keys.len());
    for key in &kana_keys {
        substring_hash.insert(key.clone(), FindWordRows::Kana(Vec::new()));
    }
    for key in &kanji_keys {
        substring_hash.insert(key.clone(), FindWordRows::Kanji(Vec::new()));
    }

    // dict.lisp:514-518 — (loop for table in '(kana-text kanji-text)
    //   for keys in ... when keys do (query ...)). Unrolled by table
    //   here so the typed `query_as<KanaText>` / `query_as<KanjiText>`
    //   stays known at compile time.
    if !kana_keys.is_empty() {
        let rows: Vec<KanaText> = ctx.store.kana_texts_by_text_any(&kana_keys).await?;
        // dict.lisp:517 — (push (cons table kt) (gethash (getf kt :text) substring-hash)).
        // CL `push` prepends, so each bucket is the reverse of the SQL
        // row order. The order is load-bearing: find-word returns the
        // bucket in this order and downstream homonym selection takes
        // the last-iterated row. Iterating the rows in reverse and
        // appending produces the same bucket order as prepending each
        // row in forward order, without shifting the bucket per insert.
        for kt in rows.into_iter().rev() {
            if let Some(FindWordRows::Kana(bucket)) = substring_hash.get_mut(&kt.text) {
                bucket.push(kt);
            }
        }
    }
    if !kanji_keys.is_empty() {
        let rows: Vec<KanjiText> = ctx.store.kanji_texts_by_text_any(&kanji_keys).await?;
        // dict.lisp:517 — reversed append mirrors CL `push` (see kana loop).
        for kt in rows.into_iter().rev() {
            if let Some(FindWordRows::Kanji(bucket)) = substring_hash.get_mut(&kt.text) {
                bucket.push(kt);
            }
        }
    }

    Ok(substring_hash)
}

/// Transliteration of `ichiran/dict:find-words-seqs` (`dict.lisp:520`).
///
/// Partitions `words` into kana vs kanji, fetches the `kanji_text` rows
/// and `kana_text` rows whose text matches and whose seq is among `seqs`,
/// and returns the kanji rows followed by the kana rows.
pub async fn find_words_seqs(
    ctx: &KaniranContext,
    words: &[&str],
    seqs: &[i32],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let mut kana_words: Vec<&str> = Vec::new();
    let mut kanji_words: Vec<&str> = Vec::new();
    for &word in words {
        if test_word(word, CharClass::Kana) {
            kana_words.push(word);
        } else {
            kanji_words.push(word);
        }
    }

    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    // dict.lisp:532 (when kanji-words (select-dao 'kanji-text ...))
    if !kanji_words.is_empty() {
        let kw: Vec<KanjiText> = ctx
            .store
            .kanji_texts_by_text_any_and_seq_any(&kanji_words, seqs)
            .await?;
        out.extend(kw.into_iter().map(KaniWordDispatchEnum::Kanji));
    }
    // dict.lisp:533 (when kana-words (select-dao 'kana-text ...))
    if !kana_words.is_empty() {
        let rw: Vec<KanaText> = ctx
            .store
            .kana_texts_by_text_any_and_seq_any(&kana_words, seqs)
            .await?;
        out.extend(rw.into_iter().map(KaniWordDispatchEnum::Kana));
    }
    Ok(out)
}

/// Port of `ichiran/dict:word-readings` (`dict.lisp:536`).
///
/// When `word` itself appears in `kana-text` the readings are just the
/// word; otherwise the kana spellings of every `kanji-text` entry with
/// that surface form, ordered by id. Returns the readings alongside
/// their romanizations.
pub async fn word_readings(
    ctx: &KaniranContext,
    word: &str,
) -> Result<(Vec<String>, Vec<String>), sqlx::Error> {
    // dict.lisp:537 (kana-seq (query (:select 'seq :from 'kana-text :where (:= 'text word)) :column))
    let kana_seq: Vec<i32> = ctx.store.kana_seqs_by_text(word).await?;
    // dict.lisp:538-545 (readings (if kana-seq (list word) …))
    let readings: Vec<String> = if !kana_seq.is_empty() {
        vec![word.to_string()]
    } else {
        // dict.lisp:540-541 (kanji-seq (query (:select 'seq :from 'kanji-text :where (:= 'text word)) :column))
        let kanji_seq: Vec<i32> = ctx.store.kanji_seqs_by_text(word).await?;
        // dict.lisp:542-545 (query (:order-by (:select 'text :from 'kana-text :where (:in 'seq (:set kanji-seq))) 'id) :column)
        ctx.store.kana_reading_texts_by_seq_any(&kanji_seq).await?
    };
    // dict.lisp:546 (values readings (mapcar #'ichiran:romanize-word readings))
    let method = RomanizationMethod::TraditionalHepburn(default_romanization_method());
    let romanizations: Vec<String> = readings
        .iter()
        .map(|reading| romanize_word(reading, method, None, true))
        .collect();
    Ok((readings, romanizations))
}

#[cfg(test)]
mod tests;
