use crate::characters::char_class::{test_word, CharClass};
use crate::conn::kani_context::KaniranContext;
use crate::dict::accessors::get_kanji;
use crate::dict::counters::methods::text;
use crate::dict::dao::KanaText;
use crate::dict::grammar::suffix::resolve::get_suffix_map;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::path::{find_word_full, CounterArg};
use crate::dict::readings::{find_word, FindWordRows};
use crate::dict::scoring::score::{compare_common, gen_score, Segment};
use crate::dict::word_info::{
    word_info_from_segment, SuffixMapTemp, WordInfo, WordInfoKana, WordInfoSeq,
};
use crate::dict::word_info_str::word_info_gloss_json;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

/// Port of `ichiran/dict:exists-reading` (`dict.lisp:1846`).
///
/// Returns the `seq` of every `kana_text` row matching `(seq, reading)`
/// — a non-empty result means the reading is recorded for that entry.
pub async fn exists_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
) -> Result<Vec<i32>, sqlx::Error> {
    ctx.store.kana_seqs_by_seq_and_text(seq, reading).await
}

/// Port of `ichiran/dict:find-word-info` (`dict.lisp:1849`).
///
/// Finds every reading for `text`, scores and sorts each as a segment,
/// converts to word-infos, and (when `reading` is given) keeps only
/// those whose kana matches or that carry `reading` as an alternate.
pub async fn find_word_info(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
    root_only: bool,
) -> Result<Vec<WordInfo>, sqlx::Error> {
    // &aux (end (length text))
    let end = text.chars().count();

    // (let ((*suffix-map-temp* (get-suffix-map text)) (*suffix-next-end* end)) …)
    // get-suffix-map borrows ctx / text; *suffix-map-temp* owns its data,
    // so materialize owned triples once.
    let suffix_map: Arc<SuffixMapTemp> = Arc::new(
        get_suffix_map(ctx, text)
            .into_iter()
            .map(|(end_pos, items)| {
                let owned: Vec<(String, String, Option<_>)> = items
                    .into_iter()
                    .map(|(substr, key, kf)| (substr.to_string(), key.to_string(), kf.cloned()))
                    .collect();
                (end_pos, owned)
            })
            .collect(),
    );
    let ctx2 = ctx
        .with_suffix_map_temp(Some(suffix_map))
        .with_suffix_next_end(Some(end as i32));

    // (all-words (if root-only (find-word text :root-only t)
    //                (find-word-full text :as-hiragana (test-word text :katakana) :counter :auto)))
    let all_words: Vec<KaniWordDispatchEnum> = if root_only {
        match find_word(&ctx2, text, true).await? {
            FindWordRows::Kana(rows) => rows.into_iter().map(KaniWordDispatchEnum::Kana).collect(),
            FindWordRows::Kanji(rows) => {
                rows.into_iter().map(KaniWordDispatchEnum::Kanji).collect()
            }
        }
    } else {
        find_word_full(
            &ctx2,
            text,
            test_word(text, CharClass::Katakana),
            Some(CounterArg::Auto),
        )
        .await?
    };

    // (segments (loop for word in all-words
    //              collect (gen-score (make-segment :start 0 :end end :word word :text text))))
    let mut segments: Vec<Segment> = Vec::with_capacity(all_words.len());
    for word in all_words {
        let mut segment = Segment {
            start: 0,
            end,
            word,
            score: None,
            info: None,
            top: None,
            text: Some(text.to_string()),
        };
        gen_score(&ctx2, &mut segment, false, &[]).await?;
        segments.push(segment);
    }

    // (segments (sort segments #'> :key #'segment-score)) — descending by score.
    segments.sort_by(|left, right| right.score.cmp(&left.score));

    // (wis (mapcar #'word-info-from-segment segments))
    let mut wis: Vec<WordInfo> = Vec::with_capacity(segments.len());
    for segment in &mut segments {
        wis.push(word_info_from_segment(&ctx2, segment).await?);
    }

    // (when reading (setf wis (loop …)))
    if let Some(reading) = reading {
        let mut filtered: Vec<WordInfo> = Vec::with_capacity(wis.len());
        for mut wi in wis {
            // for seq = (word-info-seq wi)
            let seq = wi.seq.clone();
            // if (equal (word-info-kana wi) reading) collect wi
            if matches!(&wi.kana, Some(WordInfoKana::Single(kana)) if kana == reading) {
                filtered.push(wi);
            // else if (and seq (exists-reading seq reading))
            } else if let Some(seq) = seq {
                if exists_reading_seq(&ctx2, &seq, reading).await? {
                    // do (setf (word-info-kana wi) reading) and collect wi
                    wi.kana = Some(WordInfoKana::Single(reading.to_string()));
                    filtered.push(wi);
                }
            }
        }
        wis = filtered;
    }

    Ok(wis)
}

/// `(exists-reading seq reading)` (`dict.lisp:1866`) where `seq` is the
/// `word-info-seq` slot — an int for a simple word, a list for a
/// compound. For a single seq this is the ported [`exists_reading`]
/// predicate. For a compound's list seq, postmodern renders the list as
/// a SQL row literal (`seq = (a, b, …)`), which PostgreSQL rejects with
/// `operator does not exist: integer = record` (SQLSTATE 42883);
/// reproduce the same erroring query so the failure propagates
/// identically.
async fn exists_reading_seq(
    ctx: &KaniranContext,
    seq: &WordInfoSeq,
    reading: &str,
) -> Result<bool, sqlx::Error> {
    match seq {
        WordInfoSeq::Single(single_seq) => {
            Ok(!exists_reading(ctx, *single_seq, reading).await?.is_empty())
        }
        WordInfoSeq::Multi(_) => {
            let rows = ctx
                .store
                .kana_seqs_by_seq_expr(&render_seq_row(seq), reading)
                .await?;
            Ok(!rows.is_empty())
        }
    }
}

/// Render a `word-info-seq` value the way postmodern serializes it into
/// the `(:= 'seq seq)` clause: an int as itself, a list as a
/// parenthesized comma-separated row literal (`nil` → `NULL`).
fn render_seq_row(seq: &WordInfoSeq) -> String {
    match seq {
        WordInfoSeq::Single(single_seq) => single_seq.to_string(),
        WordInfoSeq::Multi(elements) => {
            let rendered: Vec<String> = elements
                .iter()
                .map(|element| match element {
                    Some(inner) => render_seq_row(inner),
                    None => "NULL".to_string(),
                })
                .collect();
            format!("({})", rendered.join(", "))
        }
    }
}

/// Port of `ichiran/dict:find-word-info-json` (`dict.lisp:1871`).
///
/// Runs [`find_word_info`] and renders each result through
/// [`word_info_gloss_json`].
pub async fn find_word_info_json(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
    root_only: bool,
) -> Result<Vec<Value>, sqlx::Error> {
    let word_infos = find_word_info(ctx, text, reading, root_only).await?;
    let mut out = Vec::with_capacity(word_infos.len());
    for word_info in &word_infos {
        out.push(word_info_gloss_json(ctx, word_info, root_only).await?);
    }
    Ok(out)
}

/// Port of `ichiran/dict:find-word-kana-pattern` (`dict.lisp:1877`).
///
/// Selects every `kana_text` row whose `text` matches the POSIX regex
/// `pattern`, then stable-sorts the rows by [`compare_common`] over each
/// row's `common` rank (the `:null` sentinel sorts last).
pub async fn find_word_kana_pattern(
    ctx: &KaniranContext,
    pattern: &str,
) -> Result<Vec<KanaText>, sqlx::Error> {
    // (select-dao 'kana-text (:~ 'text pattern))
    let mut rows: Vec<KanaText> = ctx.store.kana_texts_by_regex(pattern).await?;
    // (stable-sort … #'compare-common :key (lambda (r) (and (not (eql (common r) :null)) (common r))))
    // — `common = None` mirrors the `:null` sentinel, so the key is the
    // row's `common` slot directly.
    rows.sort_by(|a, b| {
        let key_a = a.common.map(i64::from);
        let key_b = b.common.map(i64::from);
        if compare_common(key_a, key_b).is_truthy() {
            Ordering::Less
        } else if compare_common(key_b, key_a).is_truthy() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
    Ok(rows)
}

/// Port of `ichiran/dict:find-kanji-for-pattern` (`dict.lisp:1882`).
///
/// For each `kana_text` row matching `pattern`, collects its
/// `get-kanji` surface (when non-nil) and its `text`, then returns both
/// lists with duplicates removed keeping the first occurrence.
pub async fn find_kanji_for_pattern(
    ctx: &KaniranContext,
    pattern: &str,
) -> Result<(Vec<String>, Vec<String>), sqlx::Error> {
    let mut kanji: Vec<String> = Vec::new();
    let mut kana: Vec<String> = Vec::new();
    // (loop for r in (find-word-kana-pattern pattern) …)
    for r in find_word_kana_pattern(ctx, pattern).await? {
        let r = KaniWordDispatchEnum::Kana(r);
        // for k = (get-kanji r) / when k collect k into kanji
        if let Some(k) = get_kanji(ctx, &r).await? {
            kanji.push(k);
        }
        // collect (text r) into kana
        kana.push(text(&r).into_owned());
    }
    // (values (remove-duplicates kanji :test 'equal :from-end t)
    //         (remove-duplicates kana  :test 'equal :from-end t))
    Ok((
        remove_duplicates_from_end(kanji),
        remove_duplicates_from_end(kana),
    ))
}

// `(remove-duplicates … :test 'equal :from-end t)` — keep the first
// occurrence of each value, preserving order.
fn remove_duplicates_from_end(items: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

/// Port of `ichiran/dict:get-glosses` (`dict.lisp:1892`).
///
/// Joins `gloss` to `sense`, filters `sense.seq` to the requested set,
/// and groups rows by `seq` into `(seq, glosses)` pairs. Within each
/// group the glosses appear in reverse physical-row order.
pub async fn get_glosses(
    ctx: &KaniranContext,
    seqs: &[i32],
) -> Result<Vec<(i32, Vec<String>)>, sqlx::Error> {
    let glosses: Vec<(i32, String)> = ctx.store.glosses_by_seq_any(seqs).await?;

    let mut al: Vec<(i32, Vec<String>)> = Vec::new();
    for (seq, text) in glosses {
        // dict.lisp:1896-1899 — `if (eql (caar al) seq) do (push text (cdar al))`
        match al.last_mut() {
            Some((s, inner)) if *s == seq => inner.insert(0, text),
            _ => al.push((seq, vec![text])),
        }
    }
    Ok(al)
}

/// Port of `ichiran/dict:get-candidates` (`dict.lisp:1902`).
///
/// Returns the `entry.seq` rows matching `(text, reading)`. When
/// `text` is pure kana, the query restricts to `root_p` kana-only
/// entries (`k.text IS NULL`) whose primary kana row equals `text`;
/// otherwise it treats `text` as a kanji writing and requires both the
/// primary kanji row and the primary kana row to match.
pub async fn get_candidates(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
) -> Result<Vec<i32>, sqlx::Error> {
    let is_kana = test_word(text, CharClass::Kana);
    if is_kana {
        ctx.store.candidate_seqs_kana(text).await
    } else {
        ctx.store.candidate_seqs_kanji(text, reading).await
    }
}

/// Port of `ichiran/dict:match-glosses` (`dict.lisp:1920`).
///
/// Resolves `(text, reading)` to candidate entry seqs, then for each
/// candidate scans its glosses in DB order looking for either an
/// `update_gloss` regex match (priority 1, returns the original gloss)
/// or a match where every supplied `word` appears in the normalized
/// gloss (priority 2). First hit across `(candidate, gloss)` wins; when
/// nothing matches the first candidate is returned with `found_p =
/// false`. Empty candidates returns [`None`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchValue {
    /// Upstream returned `seq` alone — either the all-words-match arm
    /// (`found_p = true`) or the no-match fallback arm using
    /// `(car candidates)` (`found_p = false`).
    Seq(i32),
    /// Upstream returned `(list seq match)` — the `update-gloss`
    /// regex arm. `match` is the original (un-normalized) gloss
    /// string.
    SeqAndGloss(i32, String),
}

pub async fn match_glosses(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
    words: &[&str],
    normalize: Option<&dyn Fn(&str) -> String>,
    update_gloss: Option<&fancy_regex::Regex>,
) -> Result<Option<(MatchValue, bool)>, sqlx::Error> {
    let candidates = get_candidates(ctx, text, reading).await?;
    if candidates.is_empty() {
        return Ok(None);
    }
    // dict.lisp:1923 — `(nwords (mapcar normalize words))`
    let nwords: Vec<String> = words
        .iter()
        .map(|w| match normalize {
            Some(f) => f(w),
            None => (*w).to_string(),
        })
        .collect();

    // dict.lisp:1925 — `(loop for (seq . glosses) in (get-glosses candidates) ...)`
    let groups = get_glosses(ctx, &candidates).await?;
    for (seq, glosses) in groups {
        // dict.lisp:1926 — `(loop for gloss in (nreverse glosses) ...)`
        let dbo_glosses: Vec<String> = glosses.into_iter().rev().collect();
        let mut inner: Option<MatchValue> = None;
        for gloss in &dbo_glosses {
            // dict.lisp:1927 — `for ngloss = (funcall normalize gloss)`
            let ngloss = match normalize {
                Some(f) => f(gloss),
                None => gloss.clone(),
            };
            // dict.lisp:1928-1929 —
            // `when (and update-gloss (ppcre:scan update-gloss ngloss))
            //    do (return gloss)`
            if let Some(rg) = update_gloss {
                if rg
                    .is_match(&ngloss)
                    .expect("match-glosses: fancy_regex runtime error")
                {
                    inner = Some(MatchValue::SeqAndGloss(seq, gloss.clone()));
                    break;
                }
            }
            // dict.lisp:1930 —
            // `thereis (loop for word in nwords always (search word ngloss))`
            if nwords.iter().all(|w| ngloss.contains(w.as_str())) {
                inner = Some(MatchValue::Seq(seq));
                break;
            }
        }
        if let Some(m) = inner {
            return Ok(Some((m, true)));
        }
    }
    // dict.lisp:1934 — `(values (car candidates) nil)`
    Ok(Some((MatchValue::Seq(candidates[0]), false)))
}

#[cfg(test)]
mod tests;
