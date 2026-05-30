//! Port of the dict.lisp find-word layer — find-word /
//! find-substring-words / find-words-seqs / find-word-as-hiragana /
//! find-word-full / find-word-info / find-word-info-json /
//! find-word-kana-pattern / find-kanji-for-pattern / exists-reading /
//! get-candidates / get-non-arch-posi / is-arch / word-readings.

use crate::characters::char_classes::{test_word, CharClass};
use crate::characters::normalize::as_hiragana;
use crate::characters::text_utils::consecutive_char_groups;
use crate::conn::kani_context::KaniranContext;
use crate::core::methods::{default_romanization_method, RomanizationMethod};
use crate::core::romanize::romanize_word;
use crate::dict::best_path::SuffixMapTemp;
use crate::dict::best_text::get_kanji;
use crate::dict::calc_score::gen_score;
use crate::dict::counters::dispatchers::text;
use crate::dict::counters::find_counter::find_counter;
use crate::dict::dao::{KanaText, KanjiText};
use crate::dict::grammar::suffix_lookup::{find_word_suffix, get_suffix_map};
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::segment::{compare_common, subseq_slice, Segment};
use crate::dict::text_classes::{ProxyText, SimpleText};
use crate::dict::word_info::{
    word_info_from_segment, word_info_gloss_json, WordInfo, WordInfoKana, WordInfoSeq,
};
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum FindWordRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

pub async fn find_word(
    ctx: &KaniranContext,
    word: &str,
    root_only: bool,
) -> Result<FindWordRows, sqlx::Error> {
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
        return Ok(FindWordRows::Kanji(Vec::new()));
    }
    // dict.lisp:491 — (and *substring-hash* (gethash word *substring-hash*))
    if !root_only {
        if let Some(cache) = ctx.substring_hash.as_deref() {
            if let Some(rows) = cache.get(word) {
                return Ok(rows.clone());
            }
        }
    }
    let kana = test_word(word, CharClass::Kana);
    if kana {
        let rows: Vec<KanaText> = if root_only {
            sqlx::query_as(
                "SELECT wt.* FROM kana_text wt \
                 INNER JOIN entry ON wt.seq = entry.seq \
                 WHERE wt.text = $1 AND entry.root_p",
            )
            .bind(word)
            .fetch_all(&ctx.pool)
            .await?
        } else {
            sqlx::query_as("SELECT * FROM kana_text WHERE text = $1")
                .bind(word)
                .fetch_all(&ctx.pool)
                .await?
        };
        Ok(FindWordRows::Kana(rows))
    } else {
        let rows: Vec<KanjiText> = if root_only {
            sqlx::query_as(
                "SELECT wt.* FROM kanji_text wt \
                 INNER JOIN entry ON wt.seq = entry.seq \
                 WHERE wt.text = $1 AND entry.root_p",
            )
            .bind(word)
            .fetch_all(&ctx.pool)
            .await?
        } else {
            sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1")
                .bind(word)
                .fetch_all(&ctx.pool)
                .await?
        };
        Ok(FindWordRows::Kanji(rows))
    }
}

pub async fn find_substring_words(
    ctx: &KaniranContext,
    str: &str,
    sticky: &[usize],
) -> Result<SubstringHash, sqlx::Error> {
    let mut substring_hash: SubstringHash = SubstringHash::new();
    let mut kana_keys: Vec<String> = Vec::new();
    let mut kanji_keys: Vec<String> = Vec::new();

    // dict.lisp:504-512 (loop for start ... loop for end ...). CONVENTIONS
    // §4.5: cl-ppcre / subseq index by character — collect the chars
    // once so the inner slice uses character offsets.
    let chars: Vec<char> = str.chars().collect();
    let n = chars.len();

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
            // dict.lisp:510 — pre-populate hash with an empty entry,
            // then classify by kana vs. kanji.
            let is_kana = test_word(&part, CharClass::Kana);
            let empty = if is_kana {
                FindWordRows::Kana(Vec::new())
            } else {
                FindWordRows::Kanji(Vec::new())
            };
            substring_hash.insert(part.clone(), empty);
            if is_kana {
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

    // dict.lisp:514-518 — (loop for table in '(kana-text kanji-text)
    //   for keys in ... when keys do (query ...)). Unrolled by table
    //   here so the typed `query_as<KanaText>` / `query_as<KanjiText>`
    //   stays known at compile time.
    if !kana_keys.is_empty() {
        let rows: Vec<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE text = ANY($1)")
            .bind(&kana_keys)
            .fetch_all(&ctx.pool)
            .await?;
        for kt in rows {
            // dict.lisp:517 — (push (cons table kt) (gethash (getf kt :text) substring-hash)).
            // CL `push` prepends, so each bucket is the reverse of the SQL
            // row order; `insert(0, …)` mirrors it. The order is
            // load-bearing: find-word returns the bucket in this order and
            // downstream homonym selection takes the last-iterated row.
            if let Some(FindWordRows::Kana(v)) = substring_hash.get_mut(&kt.text) {
                v.insert(0, kt);
            }
        }
    }
    if !kanji_keys.is_empty() {
        let rows: Vec<KanjiText> = sqlx::query_as("SELECT * FROM kanji_text WHERE text = ANY($1)")
            .bind(&kanji_keys)
            .fetch_all(&ctx.pool)
            .await?;
        for kt in rows {
            // dict.lisp:517 — prepend to mirror CL `push` (see kana loop).
            if let Some(FindWordRows::Kanji(v)) = substring_hash.get_mut(&kt.text) {
                v.insert(0, kt);
            }
        }
    }

    Ok(substring_hash)
}

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
        let kw: Vec<KanjiText> = sqlx::query_as::<_, KanjiText>(
            "SELECT * FROM kanji_text WHERE text = ANY($1) AND seq = ANY($2)",
        )
        .bind(kanji_words.as_slice())
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        out.extend(kw.into_iter().map(KaniWordDispatchEnum::Kanji));
    }
    // dict.lisp:533 (when kana-words (select-dao 'kana-text ...))
    if !kana_words.is_empty() {
        let rw: Vec<KanaText> = sqlx::query_as::<_, KanaText>(
            "SELECT * FROM kana_text WHERE text = ANY($1) AND seq = ANY($2)",
        )
        .bind(kana_words.as_slice())
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        out.extend(rw.into_iter().map(KaniWordDispatchEnum::Kana));
    }
    Ok(out)
}

/// Boxed async finder closure for [`find_word_as_hiragana`]. Mirrors
/// `or-as-hiragana`'s `(lambda (w) (apply fn w args))` callback shape
/// (`dict-grammar.lisp:97-100`): a one-shot unary call that takes
/// the hiragana surface form and returns either a [`FindWordRows`]
/// list or a `sqlx::Error`. `Send` so the result composes with
/// `tokio::task::spawn` paths (audit harness, segmenter pipeline).
pub type HiraganaFinder<'a> = Box<
    dyn FnOnce(
            String,
        )
            -> Pin<Box<dyn Future<Output = Result<FindWordRows, sqlx::Error>> + Send + 'a>>
        + Send
        + 'a,
>;

pub async fn find_word_as_hiragana(
    ctx: &KaniranContext,
    str_: &str,
    exclude: &[i32],
    finder: Option<HiraganaFinder<'_>>,
) -> Result<Vec<ProxyText>, sqlx::Error> {
    let as_hira = as_hiragana(str_);
    if str_ == as_hira {
        return Ok(Vec::new());
    }
    let words = match finder {
        Some(f) => f(as_hira).await?,
        // root_only=true, so the substring-hash short-circuit doesn't
        // apply (find_word skips the cache check for root_only); the
        // ctx.substring_hash slot is read inside find_word.
        None => find_word(ctx, &as_hira, true).await?,
    };
    let proxies = match words {
        FindWordRows::Kana(rows) => rows
            .into_iter()
            .filter(|w| !exclude.contains(&w.seq))
            .map(|w| ProxyText {
                text: str_.to_string(),
                kana: str_.to_string(),
                source: Box::new(KaniSimpleTextDispatchEnum::Kana(w)),
                state: SimpleText::default(),
            })
            .collect(),
        FindWordRows::Kanji(rows) => rows
            .into_iter()
            .filter(|w| !exclude.contains(&w.seq))
            .map(|w| ProxyText {
                text: str_.to_string(),
                kana: str_.to_string(),
                source: Box::new(KaniSimpleTextDispatchEnum::Kanji(w)),
                state: SimpleText::default(),
            })
            .collect(),
    };
    Ok(proxies)
}

/// Closed shape of the upstream `:counter` keyword. Per CONVENTIONS
/// §4.3: the Lisp value is `nil` (absent), the keyword `:auto`, or a
/// character-index integer. `Option<CounterArg>` carries the
/// nil-vs-present distinction; the enum carries the auto-vs-integer
/// distinction.
#[derive(Debug, Clone, Copy)]
pub enum CounterArg {
    Auto,
    At(usize),
}

pub async fn find_word_full(
    ctx: &KaniranContext,
    word: &str,
    as_hiragana: bool,
    counter: Option<CounterArg>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict.lisp:1053 (find-word word)
    let simple_words_rows = find_word(ctx, word, false).await?;

    // Pre-collect simple words as KaniWordDispatchEnum values for the
    // suffix / hiragana branches that need `:matches` / `:exclude`
    // references against them.
    let simple_words: Vec<KaniWordDispatchEnum> = match &simple_words_rows {
        FindWordRows::Kana(rows) => rows
            .iter()
            .cloned()
            .map(KaniWordDispatchEnum::Kana)
            .collect(),
        FindWordRows::Kanji(rows) => rows
            .iter()
            .cloned()
            .map(KaniWordDispatchEnum::Kanji)
            .collect(),
    };

    let mut out: Vec<KaniWordDispatchEnum> = simple_words.clone();

    // dict.lisp:1055 (find-word-suffix word :matches simple-words)
    // find_word_suffix returns Vec<KaniWordDispatchEnum> directly —
    // it carries both Compound (def-simple-suffix output) and Proxy
    // (def-abbr-suffix output) variants per the etypecase at
    // dict-grammar.lisp:565-577.
    let suffix_words = find_word_suffix(ctx, word, &simple_words).await?;
    out.extend(suffix_words);

    // dict.lisp:1056-1057 (when as-hiragana (find-word-as-hiragana …))
    if as_hiragana {
        // (mapcar 'seq simple-words) — simple-words are kanji-text /
        // kana-text rows; (seq r) is the i32 slot. Mirror with a
        // direct field read keyed by variant.
        let exclude: Vec<i32> = simple_words
            .iter()
            .filter_map(|w| match w {
                KaniWordDispatchEnum::Kanji(k) => Some(k.seq),
                KaniWordDispatchEnum::Kana(k) => Some(k.seq),
                _ => None,
            })
            .collect();
        let proxies = find_word_as_hiragana(ctx, word, &exclude, None).await?;
        out.extend(proxies.into_iter().map(KaniWordDispatchEnum::Proxy));
    }

    // dict.lisp:1058-1067 (when counter …)
    if let Some(counter_arg) = counter {
        match counter_arg {
            CounterArg::Auto => {
                // dict.lisp:1060-1064 (:auto branch)
                let word_len = word.chars().count();
                let groups = consecutive_char_groups(CharClass::Number, word, 0, word_len);
                if let Some(&(g_start, g_end)) = groups.first() {
                    // (subseq word (caar groups) (cdar groups))
                    let number = subseq_slice(None, word, g_start, Some(g_end));
                    // (subseq word (cdar groups) (length word))
                    let counter_text = subseq_slice(None, word, g_end, Some(word_len));
                    let counters = find_counter(ctx, number, counter_text, None);
                    out.extend(counters.into_iter().map(KaniWordDispatchEnum::Counter));
                }
            }
            CounterArg::At(idx) => {
                // dict.lisp:1065-1067 (t branch)
                let word_len = word.chars().count();
                let number = subseq_slice(None, word, 0, Some(idx));
                let counter_text = subseq_slice(None, word, idx, Some(word_len));
                // dict.lisp:1067 (:unique (not simple-words))
                let unique = simple_words.is_empty();
                let counters = find_counter(ctx, number, counter_text, Some(unique));
                out.extend(counters.into_iter().map(KaniWordDispatchEnum::Counter));
            }
        }
    }

    Ok(out)
}

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
            let query = format!(
                "SELECT seq FROM kana_text WHERE seq = {} AND text = $1",
                render_seq_row(seq),
            );
            let rows = sqlx::query(&query)
                .bind(reading)
                .fetch_all(&ctx.pool)
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

pub async fn find_word_kana_pattern(
    ctx: &KaniranContext,
    pattern: &str,
) -> Result<Vec<KanaText>, sqlx::Error> {
    // (select-dao 'kana-text (:~ 'text pattern))
    let mut rows: Vec<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE text ~ $1")
        .bind(pattern)
        .fetch_all(&ctx.pool)
        .await?;
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

pub async fn exists_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
) -> Result<Vec<i32>, sqlx::Error> {
    let rows = sqlx::query("SELECT seq FROM kana_text WHERE seq = $1 AND text = $2")
        .bind(seq)
        .bind(reading)
        .fetch_all(&ctx.pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<i32, _>("seq"))
        .collect())
}

pub async fn get_candidates(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
) -> Result<Vec<i32>, sqlx::Error> {
    let is_kana = test_word(text, CharClass::Kana);
    if is_kana {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT e.seq FROM entry e \
             LEFT JOIN kana_text r ON e.seq = r.seq \
             LEFT JOIN kanji_text k ON e.seq = k.seq \
             WHERE e.root_p AND k.text IS NULL AND r.text = $1 AND r.ord = 0 \
             ORDER BY e.seq",
        )
        .bind(text)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    } else {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT e.seq FROM entry e \
             LEFT JOIN kana_text r ON e.seq = r.seq \
             LEFT JOIN kanji_text k ON e.seq = k.seq \
             WHERE k.text = $1 AND k.ord = 0 AND r.text = $2 AND r.ord = 0 \
             ORDER BY e.seq",
        )
        .bind(text)
        .bind(reading)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }
}

pub async fn get_non_arch_posi(
    ctx: &KaniranContext,
    seq_set: &[i32],
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT sp1.text \
         FROM sense_prop sp1 \
         LEFT JOIN sense_prop sp2 \
                ON sp1.sense_id = sp2.sense_id \
               AND sp2.tag = 'misc' \
               AND sp2.text IN ('arch', 'obsc', 'rare') \
         WHERE sp1.seq = ANY($1) \
           AND sp1.tag = 'pos' \
           AND sp2.id IS NULL",
    )
    .bind(seq_set)
    .fetch_all(&ctx.pool)
    .await
}

pub fn is_arch(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.is_arch.contains(&seq)
}

pub fn is_arch_cache(ctx: &KaniranContext) -> &HashSet<i32> {
    &ctx.is_arch
}

pub async fn build_is_arch(pool: &PgPool) -> Result<HashSet<i32>, sqlx::Error> {
    let a1: Vec<i32> = sqlx::query_scalar(
        "SELECT sense.seq FROM sense \
         LEFT JOIN sense_prop sp \
                ON sp.sense_id = sense.id \
               AND sp.tag = 'misc' \
               AND sp.text IN ('arch', 'obsc', 'rare') \
         GROUP BY sense.seq \
         HAVING bool_and(sp.id IS NOT NULL)",
    )
    .fetch_all(pool)
    .await?;
    let a2: Vec<i32> =
        sqlx::query_scalar("SELECT DISTINCT seq FROM conjugation WHERE \"from\" = ANY($1)")
            .bind(&a1)
            .fetch_all(pool)
            .await?;
    let mut set: HashSet<i32> = a1.into_iter().collect();
    set.extend(a2);
    Ok(set)
}

pub const MAX_WORD_LENGTH: usize = 50;

/// Map from a substring of an input string to the `kana_text` /
/// `kanji_text` rows pre-fetched for it by `find-substring-words`.
/// Per-key uniformity (all rows from one table) is enforced by the
/// populator's kana-vs-kanji key split (`dict.lisp:511`).
pub type SubstringHash = HashMap<String, FindWordRows>;

pub async fn word_readings(
    ctx: &KaniranContext,
    word: &str,
) -> Result<(Vec<String>, Vec<String>), sqlx::Error> {
    // dict.lisp:537 (kana-seq (query (:select 'seq :from 'kana-text :where (:= 'text word)) :column))
    let kana_seq: Vec<i32> = sqlx::query_scalar("SELECT seq FROM kana_text WHERE text = $1")
        .bind(word)
        .fetch_all(&ctx.pool)
        .await?;
    // dict.lisp:538-545 (readings (if kana-seq (list word) …))
    let readings: Vec<String> = if !kana_seq.is_empty() {
        vec![word.to_string()]
    } else {
        // dict.lisp:540-541 (kanji-seq (query (:select 'seq :from 'kanji-text :where (:= 'text word)) :column))
        let kanji_seq: Vec<i32> = sqlx::query_scalar("SELECT seq FROM kanji_text WHERE text = $1")
            .bind(word)
            .fetch_all(&ctx.pool)
            .await?;
        // dict.lisp:542-545 (query (:order-by (:select 'text :from 'kana-text :where (:in 'seq (:set kanji-seq))) 'id) :column)
        sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = ANY($1) ORDER BY id")
            .bind(&kanji_seq)
            .fetch_all(&ctx.pool)
            .await?
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
mod test_find_substring_words {
    //! Per-key buckets cross-checked against the local ichiran Postgres
    //! (2026-05-25), the same DB these tests query. Each bucket is
    //! compared as a sorted `(seq, ord, common)` list: the populating
    //! query (`text = ANY(...)`) has no ORDER BY, so the bucket order is
    //! not stable — sorting both sides keeps the comparison
    //! order-independent. Test threads must be 1 — `cargo test --
    //! --test-threads=1` per the project's DB-test convention.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// A key's bucket as a sorted `(seq, ord, common)` list. Both this
    /// and the expected literal are in seq order so the unordered SQL
    /// bucket can't make the comparison flake.
    fn rows_sorted(h: &SubstringHash, key: &str) -> Vec<(i32, i32, Option<i32>)> {
        let mut out: Vec<(i32, i32, Option<i32>)> =
            match h.get(key).unwrap_or_else(|| panic!("missing key {key:?}")) {
                FindWordRows::Kana(v) => v.iter().map(|r| (r.seq, r.ord, r.common)).collect(),
                FindWordRows::Kanji(v) => v.iter().map(|r| (r.seq, r.ord, r.common)).collect(),
            };
        out.sort();
        out
    }

    fn keys_sorted(h: &SubstringHash) -> Vec<String> {
        let mut ks: Vec<String> = h.keys().cloned().collect();
        ks.sort();
        ks
    }

    fn is_kana(h: &SubstringHash, key: &str) -> bool {
        matches!(h.get(key), Some(FindWordRows::Kana(_)))
    }

    // 'こ' and 'ね' buckets are each shared by two tests — one copy here.
    fn ko_rows() -> Vec<(i32, i32, Option<i32>)> {
        vec![
            (1264740, 0, Some(0)),
            (1267110, 0, None),
            (1307770, 0, Some(1)),
            (1504770, 1, None),
            (1531190, 1, None),
            (1659920, 0, None),
            (1956240, 0, Some(28)),
            (2065150, 1, None),
            (2087990, 0, None),
            (2153770, 0, Some(0)),
            (2215030, 0, None),
            (2230390, 0, None),
            (2577750, 0, None),
            (2788170, 0, None),
            (2842951, 0, None),
            (2844354, 0, None),
        ]
    }

    fn ne_rows() -> Vec<(i32, i32, Option<i32>)> {
        vec![
            (1290020, 0, Some(5)),
            (1307780, 0, Some(0)),
            (1642760, 0, Some(15)),
            (2029080, 0, Some(0)),
            (2836242, 0, None),
            (2841117, 3, None),
            (2859162, 0, Some(0)),
            (10426293, 0, None),
        ]
    }

    #[tokio::test]
    async fn single_kanji_char_one_key() {
        // '猫' (no sticky): one kanji-classified key, two rows.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫", &[]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["猫".to_string()]);
        assert!(!is_kana(&h, "猫"), "'猫' should be kanji variant");
        assert_eq!(
            rows_sorted(&h, "猫"),
            vec![(1467640, 0, Some(7)), (2698030, 0, None)]
        );
    }

    #[tokio::test]
    async fn mixed_kana_kanji_three_keys() {
        // '猫が': が (7 kana), 猫 (2 kanji), 猫が (empty, kanji-classified
        // — the mixed string contains a kanji).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["が".to_string(), "猫".to_string(), "猫が".to_string()]
        );
        assert!(is_kana(&h, "が"), "'が' should be kana variant");
        assert_eq!(
            rows_sorted(&h, "が"),
            vec![
                (1197760, 0, Some(40)),
                (1202270, 1, None),
                (2028930, 0, Some(0)),
                (2220800, 0, None),
                (2224630, 0, None),
                (2232110, 0, None),
                (2834041, 0, None),
            ]
        );
        assert!(!is_kana(&h, "猫"), "'猫' should be kanji variant");
        assert_eq!(
            rows_sorted(&h, "猫"),
            vec![(1467640, 0, Some(7)), (2698030, 0, None)]
        );
        assert!(!is_kana(&h, "猫が"), "mixed substring classified non-kana");
        assert!(rows_sorted(&h, "猫が").is_empty());
    }

    #[tokio::test]
    async fn sticky_end_blocks_substrings() {
        // '猫が' sticky=(1): every 1-char substring starts or ends at
        // pos 1, so only the length-2 key survives (empty bucket).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[1]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["猫が".to_string()]);
        assert!(rows_sorted(&h, "猫が").is_empty());
    }

    #[tokio::test]
    async fn sticky_start_and_end_block() {
        // 'ねこが' sticky=(0 3): start=0 and end=3 blocked, so only 'こ'
        // (start=1, end=2) survives.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[0, 3]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["こ".to_string()]);
        assert_eq!(rows_sorted(&h, "こ"), ko_rows());
    }

    #[tokio::test]
    async fn sticky_interior_blocks_boundary_only() {
        // 'ねこが' sticky=(2): ね (8), こが (6), ねこが (empty). start=2
        // and end=2 are both blocked.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[2]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["こが".to_string(), "ね".to_string(), "ねこが".to_string()]
        );
        assert_eq!(
            rows_sorted(&h, "こが"),
            vec![
                (1265180, 0, None),
                (1265190, 0, None),
                (10136364, 0, None),
                (10276500, 0, None),
                (12294787, 0, None),
                (12295833, 0, None),
            ]
        );
        assert_eq!(rows_sorted(&h, "ね"), ne_rows());
        assert!(rows_sorted(&h, "ねこが").is_empty());
    }

    #[tokio::test]
    async fn empty_string_empty_hash() {
        // REPL '' empty: n keys=0
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "", &[]).await.unwrap();
        assert!(h.is_empty());
    }

    #[tokio::test]
    async fn ascii_unknown_pre_seeds_empty_entry() {
        // 'x': one kanji-classified key, empty bucket — the pre-seeded
        // empty entry survives the no-row query.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "x", &[]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["x".to_string()]);
        assert!(
            !is_kana(&h, "x"),
            "'x' classified kanji (not in kana char set)"
        );
        assert!(rows_sorted(&h, "x").is_empty());
    }

    #[tokio::test]
    async fn full_kana_three_keys() {
        // 'ねこ': こ (16), ね (8), ねこ (1).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこ", &[]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["こ".to_string(), "ね".to_string(), "ねこ".to_string()]
        );
        assert_eq!(rows_sorted(&h, "こ"), ko_rows());
        assert_eq!(rows_sorted(&h, "ね"), ne_rows());
        assert_eq!(rows_sorted(&h, "ねこ"), vec![(1467640, 0, Some(7))]);
    }

    /// Order guard for the `insert(0, …)` prepend (dict.lisp:517 `push`):
    /// a multi-row bucket must be the *reverse* of the database's row
    /// order, not its fetch order. Compared unsorted, unlike the other
    /// tests here — that's the point. DB-agnostic: derives the expected
    /// order from the same query the populator runs, so it pins the
    /// reversal relationship rather than hard-coded seqs. '行って' has
    /// 3 kanji rows on the local DB.
    #[tokio::test]
    async fn bucket_is_reverse_of_fetch_order() {
        let ctx = ctx_from_env().await;
        let keys = vec!["行って".to_string()];
        let fetch: Vec<i32> = sqlx::query_scalar("SELECT seq FROM kanji_text WHERE text = ANY($1)")
            .bind(&keys)
            .fetch_all(&ctx.pool)
            .await
            .unwrap();
        assert!(fetch.len() > 1, "test needs a multi-row bucket");
        let h = find_substring_words(&ctx, "行って", &[]).await.unwrap();
        let bucket: Vec<i32> = match h.get("行って").unwrap() {
            FindWordRows::Kanji(v) => v.iter().map(|r| r.seq).collect(),
            FindWordRows::Kana(v) => v.iter().map(|r| r.seq).collect(),
        };
        let mut expected = fetch;
        expected.reverse();
        assert_eq!(bucket, expected, "bucket must be reverse of fetch order");
    }
}

#[cfg(test)]
mod test_find_words_seqs {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn describe(word: &KaniWordDispatchEnum) -> (&'static str, i32, &str) {
        match word {
            KaniWordDispatchEnum::Kanji(k) => ("kanji", k.seq, k.text.as_str()),
            KaniWordDispatchEnum::Kana(k) => ("kana", k.seq, k.text.as_str()),
            _ => panic!("find_words_seqs must only return kanji-text / kana-text"),
        }
    }

    /// REPL (.103, `ichiran/dict::find-words-seqs`), 2026-05-24. Each case
    /// returns one row: a kanji word fills `kw` (kana-words empty), a kana
    /// word fills `rw` (kanji-words empty).
    #[tokio::test]
    async fn single_row_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&[&str], &[i32], (&str, i32, &str))] = &[
            (&["食べる"], &[1358280], ("kanji", 1358280, "食べる")),
            (&["たべる"], &[1358280], ("kana", 1358280, "たべる")),
            (&["見る"], &[1259290], ("kanji", 1259290, "見る")),
        ];
        for (words, seqs, expected) in cases {
            let result = find_words_seqs(&ctx, words, seqs).await.unwrap();
            assert_eq!(result.len(), 1, "words={words:?}");
            assert_eq!(describe(&result[0]), *expected, "words={words:?}");
        }
    }

    /// REPL: `(find-words-seqs "みる" '(1213770 1259290 1365450 1772790
    /// 2255060 10553286))` → 6 KANA-TEXT rows, one per matching seq.
    #[tokio::test]
    async fn kana_multi_seq() {
        let ctx = ctx().await;
        let seqs = [1213770, 1259290, 1365450, 1772790, 2255060, 10553286];
        let result = find_words_seqs(&ctx, &["みる"], &seqs).await.unwrap();
        assert_eq!(result.len(), 6);
        let mut got: Vec<i32> = result
            .iter()
            .map(|word| {
                let (kind, seq, text) = describe(word);
                assert_eq!((kind, text), ("kana", "みる"));
                seq
            })
            .collect();
        got.sort_unstable();
        assert_eq!(got, seqs);
    }

    /// REPL: `(find-words-seqs '("食べる" "たべる") '(1358280))` → KANJI-TEXT
    /// 食べる then KANA-TEXT たべる. Exercises `(nconc kw rw)` with both
    /// partitions non-empty.
    #[tokio::test]
    async fn mixed_two_words() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["食べる", "たべる"], &[1358280])
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(describe(&result[0]), ("kanji", 1358280, "食べる"));
        assert_eq!(describe(&result[1]), ("kana", 1358280, "たべる"));
    }

    /// REPL: `(find-words-seqs '("見る" "みる" "食べる") '(1259290 1358280))`
    /// → KANJI 見る, KANJI 食べる, KANA みる. `(nconc kw rw)` guarantees all
    /// kanji rows precede all kana rows; intra-partition order is the DB's.
    #[tokio::test]
    async fn mixed_partition() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["見る", "みる", "食べる"], &[1259290, 1358280])
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        let kana_start = result
            .iter()
            .position(|word| matches!(word, KaniWordDispatchEnum::Kana(_)))
            .unwrap();
        assert!(result[..kana_start]
            .iter()
            .all(|word| matches!(word, KaniWordDispatchEnum::Kanji(_))));
        assert!(result[kana_start..]
            .iter()
            .all(|word| matches!(word, KaniWordDispatchEnum::Kana(_))));
        let mut got: Vec<(&str, i32, &str)> = result.iter().map(describe).collect();
        got.sort_unstable();
        let mut expected = vec![
            ("kanji", 1259290, "見る"),
            ("kanji", 1358280, "食べる"),
            ("kana", 1259290, "みる"),
        ];
        expected.sort_unstable();
        assert_eq!(got, expected);
    }

    /// REPL: `(find-words-seqs "食べる" 9999999)` → NIL. Word matches a
    /// row but no row carries the seq, so the `seq = ANY` filter empties it.
    #[tokio::test]
    async fn no_match_seq() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["食べる"], &[9999999])
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    /// Empty `words` leaves both `kanji-words` and `kana-words` nil, so the
    /// two `(when ...)` guards skip every query and `(nconc nil nil)` is nil.
    #[tokio::test]
    async fn empty_words() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &[], &[1358280]).await.unwrap();
        assert!(result.is_empty());
    }
}

#[cfg(test)]
mod test_find_word_full {
    use super::*;
    use crate::dict::text_classes::ScoreMod;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(find-word-full "区別")` → 1 KANJI-TEXT (seq=1244250).
    /// Single simple-text, no suffix / hiragana / counter branches.
    #[tokio::test]
    async fn t1_simple_kanji_word() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "区別", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1244250);
        assert_eq!(k.text, "区別");
    }

    /// REPL: `(find-word-full "私")` → 14 KANJI-TEXT rows (polysemous
    /// 私). Exercises multi-row simple-words.
    #[tokio::test]
    async fn t2_polysemous_kanji() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "私", false, None).await.unwrap();
        assert_eq!(r.len(), 14);
        for w in &r {
            assert!(matches!(w, KaniWordDispatchEnum::Kanji(_)));
        }
    }

    /// REPL: `(find-word-full "勉強する")` → 1 COMPOUND-TEXT.
    /// simple-words for 勉強する is empty; suffix-suru fires through
    /// the partial `*suffix-list*` (suru row registered) and produces
    /// 1 compound (勉強+する).
    #[tokio::test]
    async fn t3_suru_suffix_via_registered_row() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "勉強する", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND-TEXT");
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// REPL: `(find-word-full "我々ら")` → 1 COMPOUND-TEXT via the
    /// `ra` suffix row.
    #[tokio::test]
    async fn t4_ra_suffix_via_registered_row() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "我々ら", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND-TEXT");
        };
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
    }

    /// REPL: `(find-word-full "食べてる")` → 1 COMPOUND-TEXT
    /// text="食べてる" kana="たべてる" via `suffix-teiru`. primary =
    /// KANJI-TEXT 食べて (seq=10092233), words = (primary, KANA-TEXT
    /// いる seq=1577980), score_mod=3, score_base=nil.
    #[tokio::test]
    async fn t5_teiru_suffix_compound() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "食べてる", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected Compound, got {:?}", r[0]);
        };
        assert_eq!(c.text, "食べてる");
        assert_eq!(c.kana, "たべてる");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        let KaniWordDispatchEnum::Kanji(primary) = &*c.primary else {
            panic!("expected Kanji primary, got {:?}", c.primary);
        };
        assert_eq!(primary.seq, 10092233);
        assert_eq!(primary.text, "食べて");
        assert_eq!(c.words.len(), 2);
        let KaniWordDispatchEnum::Kanji(w0) = &c.words[0] else {
            panic!("expected Kanji words[0]");
        };
        assert_eq!(w0.seq, 10092233);
        let KaniWordDispatchEnum::Kana(w1) = &c.words[1] else {
            panic!("expected Kana words[1]");
        };
        assert_eq!(w1.seq, 1577980);
        assert_eq!(w1.text, "いる");
    }

    /// REPL: `(find-word-full "xyzabc")` → NIL. No simple-text, no
    /// suffix expansion via the cache.
    #[tokio::test]
    async fn t6_no_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "xyzabc", false, None).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-full "ジャバスクリプト" :as-hiragana t)` → 1
    /// (the existing kana_text row 2302400; the hiragana fallback
    /// excludes the same seq, so no proxies added).
    #[tokio::test]
    async fn t7_as_hiragana_with_existing_kana_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "ジャバスクリプト", true, None)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kana(k) = &r[0] else {
            panic!("expected KANA-TEXT");
        };
        assert_eq!(k.seq, 2302400);
    }

    /// REPL: `(find-word-full "ハイ" :as-hiragana t)` → 14:
    ///   1 KANA-TEXT (the existing ハイ row) + 13 PROXY-TEXT (the
    ///   13 はい kana_text root rows wrapped as proxies).
    #[tokio::test]
    async fn t8_as_hiragana_with_proxy_fallback() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "ハイ", true, None).await.unwrap();
        assert_eq!(r.len(), 14);
        let kana_count = r
            .iter()
            .filter(|w| matches!(w, KaniWordDispatchEnum::Kana(_)))
            .count();
        let proxy_count = r
            .iter()
            .filter(|w| matches!(w, KaniWordDispatchEnum::Proxy(_)))
            .count();
        assert_eq!(kana_count, 1);
        assert_eq!(proxy_count, 13);
    }

    /// REPL: `(find-word-full "三本" :counter :auto)` → 3:
    ///   1 KANJI-TEXT (existing 三本) + 1 COUNTER-TEXT + 1
    ///   COUNTER-HIFUMI. Exercises the `:auto` branch through
    ///   `consecutive-char-groups`.
    #[tokio::test]
    async fn t9_counter_auto_with_simple_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "三本", false, Some(CounterArg::Auto))
            .await
            .unwrap();
        assert_eq!(r.len(), 3);
        assert!(matches!(r[0], KaniWordDispatchEnum::Kanji(_)));
        assert!(matches!(r[1], KaniWordDispatchEnum::Counter(_)));
        assert!(matches!(r[2], KaniWordDispatchEnum::Counter(_)));
    }

    /// REPL: `(find-word-full "5本" :counter 1)` → 2 COUNTER-TEXT
    /// (number text "5", counter unit "本"). Integer-index branch;
    /// simple-words is empty so `:unique` resolves to T.
    #[tokio::test]
    async fn t10_counter_explicit_index() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "5本", false, Some(CounterArg::At(1)))
            .await
            .unwrap();
        assert_eq!(r.len(), 2);
        for w in &r {
            assert!(matches!(w, KaniWordDispatchEnum::Counter(_)));
        }
    }

    /// REPL: `(find-word-full "区別" :counter :auto)` → 1 (just the
    /// kanji-text; `consecutive-char-groups :number` returns NIL for
    /// 区別 → counter branch contributes nothing).
    #[tokio::test]
    async fn t11_counter_auto_no_number_group() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "区別", false, Some(CounterArg::Auto))
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        assert!(matches!(r[0], KaniWordDispatchEnum::Kanji(_)));
    }

    /// REPL: `(find-word-full <long>)` → 0 (full result, not just the
    /// `find-word` branch). The `*max-word-length*` gate inside
    /// `find-word` short-circuits the simple-words path; the
    /// `find-word-suffix` branch still runs but finds no cache hit on
    /// this specific 51-char hiragana run — REPL-verified against
    /// both the random-hiragana string below and a realistic
    /// over-length sentence.
    #[tokio::test]
    async fn t12_over_length_short_circuit() {
        let ctx = ctx().await;
        let long = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんがぎぐげござ";
        let r = find_word_full(&ctx, long, false, None).await.unwrap();
        assert!(r.is_empty());
    }
}

#[cfg(test)]
mod test_find_word_info {
    //! Ground truth captured from `ichiran/dict:find-word-info` on .103
    //! (2026-05-25) with `(init-suffixes t t)` forced first so the suffix
    //! cache is fully populated — matching `KaniranContext::from_env`'s
    //! eager populate. Tests run against the local DB per project policy.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn kana_of(wi: &WordInfo) -> &str {
        match &wi.kana {
            Some(WordInfoKana::Single(k)) => k,
            other => panic!("expected single kana, got {other:?}"),
        }
    }

    fn single_seq(wi: &WordInfo) -> i32 {
        match &wi.seq {
            Some(WordInfoSeq::Single(s)) => *s,
            other => panic!("expected single seq, got {other:?}"),
        }
    }

    /// One-result lookups: every populated WordInfo field per REPL.
    /// Covers the kanji branch, the katakana `:as-hiragana` branch
    /// (ヨーロッパ / コンピューター resolve to a KANA row), and
    /// ASCII-digit absence of the counter path. For all simple words
    /// `true-text` equals the surface text and find-word-info sets
    /// `start`=0 / `end`=(length text).
    #[tokio::test]
    async fn single_result_cases() {
        use crate::dict::word_info::WordInfoType;
        let ctx = ctx().await;
        // (text, kana, seq, score, is_kana_type)
        let cases: &[(&str, &str, i32, i32, bool)] = &[
            ("政府", "せいふ", 1376070, 325, false),
            ("経済", "けいざい", 1251320, 325, false),
            ("今日", "きょう", 1579110, 312, false),
            ("明日", "あした", 1584660, 273, false),
            ("ヨーロッパ", "ヨーロッパ", 1137570, 384, true),
            ("コンピューター", "コンピューター", 1053350, 440, true),
        ];
        for (text, kana, seq, score, is_kana) in cases {
            let result = find_word_info(&ctx, text, None, false).await.unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            let wi = &result[0];
            assert_eq!(&wi.text, text, "text={text}");
            assert_eq!(kana_of(wi), *kana, "text={text}");
            assert_eq!(single_seq(wi), *seq, "text={text}");
            assert_eq!(wi.score, Some(*score), "text={text}");
            assert_eq!(
                wi.kind,
                if *is_kana {
                    WordInfoType::Kana
                } else {
                    WordInfoType::Kanji
                },
                "text={text}"
            );
            assert_eq!(wi.true_text.as_deref(), Some(*text), "text={text}");
            assert_eq!(wi.start, Some(0), "text={text}");
            assert_eq!(wi.end, Some(text.chars().count()), "text={text}");
            assert!(wi.counter.is_none(), "text={text}");
            assert!(wi.components.is_empty(), "text={text}");
        }
    }

    /// Multi-result lookups with distinct scores: the `(sort … #'>)`
    /// orders strictly descending. 何 (なに 24 / なん 16), 一人 (312 /
    /// 208), 二人 (325 / 208).
    #[tokio::test]
    async fn multi_result_sorted_descending() {
        let ctx = ctx().await;
        // (text, [(kana, seq, score), …] in expected order)
        let cases: &[(&str, &[(&str, i32, i32)])] = &[
            ("何", &[("なに", 1577100, 24), ("なん", 2846738, 16)]),
            (
                "一人",
                &[("ひとり", 1576150, 312), ("ひとり", 2149890, 208)],
            ),
            (
                "二人",
                &[("ふたり", 1582670, 325), ("ふたり", 2149890, 208)],
            ),
        ];
        for (text, expected) in cases {
            let result = find_word_info(&ctx, text, None, false).await.unwrap();
            assert_eq!(result.len(), expected.len(), "text={text}");
            for (wi, (kana, seq, score)) in result.iter().zip(expected.iter()) {
                assert_eq!(&wi.text, text, "text={text}");
                assert_eq!(kana_of(wi), *kana, "text={text}");
                assert_eq!(single_seq(wi), *seq, "text={text}");
                assert_eq!(wi.score, Some(*score), "text={text}");
            }
        }
    }

    /// 三本 → 3 results, two tied at score 208 (DB-order between the two
    /// ties is unspecified per docs/known_issues.md) then みもと 143. Assert the
    /// descending score sequence and the seq set, not the tie order.
    #[tokio::test]
    async fn score_tie_then_lower() {
        let ctx = ctx().await;
        let result = find_word_info(&ctx, "三本", None, false).await.unwrap();
        assert_eq!(result.len(), 3);
        let scores: Vec<i32> = result.iter().map(|wi| wi.score.unwrap()).collect();
        assert_eq!(scores, vec![208, 208, 143]);
        let mut seqs: Vec<i32> = result.iter().map(single_seq).collect();
        seqs.sort_unstable();
        assert_eq!(seqs, vec![1260670, 1301640, 1522150]);
        assert_eq!(single_seq(&result[2]), 1260670); // the 143 row sorts last
    }

    /// 5個 → 2 counter-text readings (ごこ 128 / ごか 40). Exercises the
    /// `:counter :auto` branch of find-word-full and a Single (source)
    /// seq on a counter word.
    #[tokio::test]
    async fn counter_auto_results() {
        let ctx = ctx().await;
        let result = find_word_info(&ctx, "5個", None, false).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            (kana_of(&result[0]), single_seq(&result[0]), result[0].score),
            ("ごこ", 1264740, Some(128))
        );
        assert_eq!(
            (kana_of(&result[1]), single_seq(&result[1]), result[1].score),
            ("ごか", 2220320, Some(40))
        );
        // counter-text: true-text nil, counter = (value-string, ordinalp), start/end 0/2.
        for wi in &result {
            assert_eq!(wi.counter, Some(("Value: 5".to_string(), false)));
            assert!(wi.true_text.is_none());
            assert_eq!(wi.start, Some(0));
            assert_eq!(wi.end, Some(2));
        }
    }

    /// `:root-only t` routes all-words through `(find-word text :root-only t)`.
    #[tokio::test]
    async fn root_only_cases() {
        let ctx = ctx().await;
        let cases: &[(&str, &str, i32, i32)] = &[
            ("経済", "けいざい", 1251320, 325),
            ("三本", "さんぼん", 1301640, 208),
            ("一人", "ひとり", 1576150, 312),
        ];
        for (text, kana, seq, score) in cases {
            let result = find_word_info(&ctx, text, None, true).await.unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            assert_eq!(kana_of(&result[0]), *kana, "text={text}");
            assert_eq!(single_seq(&result[0]), *seq, "text={text}");
            assert_eq!(result[0].score, Some(*score), "text={text}");
        }
    }

    /// reading matches the word's kana (`(equal (word-info-kana wi) reading)`)
    /// → collect unchanged. Includes the compound 食べてる whose kana
    /// たべてる matches before the list-seq branch is reached.
    #[tokio::test]
    async fn reading_match_collects() {
        let ctx = ctx().await;
        let seifu = find_word_info(&ctx, "政府", Some("せいふ"), false)
            .await
            .unwrap();
        assert_eq!(seifu.len(), 1);
        assert_eq!(kana_of(&seifu[0]), "せいふ");
        assert_eq!(single_seq(&seifu[0]), 1376070);

        let taberu = find_word_info(&ctx, "食べてる", Some("たべてる"), false)
            .await
            .unwrap();
        assert_eq!(taberu.len(), 1);
        assert_eq!(kana_of(&taberu[0]), "たべてる");
        assert_eq!(
            taberu[0].seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10092233)),
                Some(WordInfoSeq::Single(1577980)),
            ]))
        );
    }

    /// reading differs from the kana but `exists-reading` finds it for the
    /// seq → relabel the kana to reading and collect. Mismatched rows whose
    /// seq lacks the reading are dropped (一人 keeps only seq 1576150).
    #[tokio::test]
    async fn reading_relabel_and_drop() {
        let ctx = ctx().await;
        // (text, reading, expected (kana, seq, score))
        let cases: &[(&str, &str, &str, i32, i32)] = &[
            ("一人", "いちにん", "いちにん", 1576150, 312),
            ("今日", "こんにち", "こんにち", 1579110, 312),
            ("今日", "こんじつ", "こんじつ", 1579110, 312),
            ("二人", "ににん", "ににん", 1582670, 325),
            ("何", "なん", "なん", 2846738, 16),
        ];
        for (text, reading, kana, seq, score) in cases {
            let result = find_word_info(&ctx, text, Some(reading), false)
                .await
                .unwrap();
            assert_eq!(result.len(), 1, "text={text} reading={reading}");
            assert_eq!(kana_of(&result[0]), *kana, "text={text} reading={reading}");
            assert_eq!(
                single_seq(&result[0]),
                *seq,
                "text={text} reading={reading}"
            );
            assert_eq!(
                result[0].score,
                Some(*score),
                "text={text} reading={reading}"
            );
        }
    }

    /// reading matches no row and `exists-reading` is empty for every seq
    /// → every word-info dropped, result empty.
    #[tokio::test]
    async fn reading_drops_all() {
        let ctx = ctx().await;
        assert!(find_word_info(&ctx, "政府", Some("ありえない"), false)
            .await
            .unwrap()
            .is_empty());
        assert!(find_word_info(&ctx, "何", Some("ぜんぜんちがう"), false)
            .await
            .unwrap()
            .is_empty());
    }

    /// Compounds carry a list seq and a per-part `components` list (each
    /// child a WordInfo with `primary` set iff its seq is the compound's
    /// primary). `true-text` is nil, start/end span the whole text.
    #[tokio::test]
    async fn compound_results() {
        let ctx = ctx().await;
        // (text, kana, score, [(comp_text, comp_kana, comp_seq, primary)])
        let cases: &[(&str, &str, i32, &[(&str, &str, i32, bool)])] = &[
            (
                "食べてる",
                "たべてる",
                434,
                &[
                    ("食べて", "たべて", 10092233, true),
                    ("いる", "いる", 1577980, false),
                ],
            ),
            (
                "勉強する",
                "べんきょう する",
                736,
                &[
                    ("勉強", "べんきょう", 1512670, true),
                    ("する", "する", 1157170, false),
                ],
            ),
        ];
        for (text, kana, score, comps) in cases {
            let result = find_word_info(&ctx, text, None, false).await.unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            let wi = &result[0];
            assert_eq!(&wi.text, text, "text={text}");
            assert_eq!(kana_of(wi), *kana, "text={text}");
            assert_eq!(wi.score, Some(*score), "text={text}");
            assert!(wi.true_text.is_none(), "text={text}");
            assert_eq!(wi.start, Some(0), "text={text}");
            assert_eq!(wi.end, Some(text.chars().count()), "text={text}");
            let expected_seq = WordInfoSeq::Multi(
                comps
                    .iter()
                    .map(|(_, _, s, _)| Some(WordInfoSeq::Single(*s)))
                    .collect(),
            );
            assert_eq!(wi.seq, Some(expected_seq), "text={text}");
            assert_eq!(wi.components.len(), comps.len(), "text={text}");
            for (comp, (comp_text, comp_kana, comp_seq, primary)) in
                wi.components.iter().zip(comps.iter())
            {
                assert_eq!(&comp.text, comp_text, "text={text}");
                assert_eq!(kana_of(comp), *comp_kana, "text={text}");
                assert_eq!(single_seq(comp), *comp_seq, "text={text}");
                assert_eq!(comp.primary, *primary, "text={text}");
            }
        }
    }

    /// A compound whose kana ≠ reading reaches `(exists-reading seq reading)`
    /// with a list seq; upstream errors with `integer = record` (SQLSTATE
    /// 42883), so the port returns the same DB error.
    #[tokio::test]
    async fn compound_reading_mismatch_errors() {
        let ctx = ctx().await;
        let err = find_word_info(&ctx, "食べてる", Some("ちがうよみ"), false)
            .await
            .expect_err("compound list-seq exists-reading must raise a DB error");
        match err {
            sqlx::Error::Database(db) => assert_eq!(db.code().as_deref(), Some("42883")),
            other => panic!("expected SQLSTATE 42883 database error, got {other:?}"),
        }
    }

    /// No dictionary hit → empty result.
    #[tokio::test]
    async fn no_match_is_empty() {
        let ctx = ctx().await;
        assert!(find_word_info(&ctx, "qwxz", None, false)
            .await
            .unwrap()
            .is_empty());
    }
}

#[cfg(test)]
mod test_find_word_info_json {
    //! Ground truth from `(jsown:to-json (find-word-info-json …))` on .103
    //! (2026-05-25) after `(init-suffixes t t)`. jsown's `\uXXXX` decoded to
    //! the raw-UTF-8 serde_json emits. Local DB per project policy.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// Maps word-info-gloss-json over find-word-info. Covers a single-result
    /// noun, the multi-result counter (two objects), root-only (one object,
    /// no conj), and root-only on a conjugated compound (no root entry →
    /// empty list).
    #[tokio::test]
    async fn find_word_info_json_cases() {
        let ctx = ctx_from_env().await;
        // (text, reading, root_only, expected list json)
        let cases: &[(&str, Option<&str>, bool, &str)] = &[
            (
                "経済",
                None,
                false,
                r#"[{"reading":"経済 【けいざい】","text":"経済","kana":"けいざい","score":325,"seq":1251320,"gloss":[{"pos":"[n]","gloss":"economy; economics"},{"pos":"[n]","gloss":"finance; (one's) finances; financial circumstances"},{"pos":"[n]","gloss":"being economical; economy; thrift"}],"conj":[]}]"#,
            ),
            (
                "経済",
                None,
                true,
                r#"[{"reading":"経済 【けいざい】","text":"経済","kana":"けいざい","score":325,"seq":1251320,"gloss":[{"pos":"[n]","gloss":"economy; economics"},{"pos":"[n]","gloss":"finance; (one's) finances; financial circumstances"},{"pos":"[n]","gloss":"being economical; economy; thrift"}]}]"#,
            ),
            ("行きたい", None, true, "[]"),
        ];
        for (text, reading, root_only, expected) in cases {
            let result = find_word_info_json(&ctx, text, *reading, *root_only)
                .await
                .unwrap();
            assert_eq!(json(&result), *expected, "text={text} root={root_only}");
        }
    }

    /// `:reading` restricts/relabels: 今日 with こんにち keeps the seq whose
    /// reading exists, relabeling the word-info kana (mirrors find-word-info's
    /// reading branch) before serialization.
    #[tokio::test]
    async fn reading_relabel() {
        let ctx = ctx_from_env().await;
        let result = find_word_info_json(&ctx, "今日", Some("こんにち"), false)
            .await
            .unwrap();
        assert_eq!(
            json(&result),
            r#"[{"reading":"今日 【こんにち】","text":"今日","kana":"こんにち","score":312,"seq":1579110,"gloss":[{"pos":"[n,adv]","gloss":"today; this day"},{"pos":"[n,adv]","gloss":"these days; recently; nowadays"}],"conj":[]}]"#
        );
    }
}

#[cfg(test)]
mod test_find_word_kana_pattern {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::find-word-kana-pattern`), 2026-05-25.
    /// Asserts the ordered `common` sequence each pattern yields (the
    /// values are deterministic; tied-`common` rows keep their DB scan
    /// order). `^はし$` exercises positive-ascending-then-null ordering
    /// across six homophones (5, 5, 19, null, null, null); `^あれ$`
    /// exercises the `0` rank sorting after positives but before nulls
    /// (21, 0, null, null); `^xyzzlkj$` matches nothing.
    #[tokio::test]
    async fn common_sort_order() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str, Vec<Option<i32>>)] = &[
            (
                "^はし$",
                "はし",
                vec![Some(5), Some(5), Some(19), None, None, None],
            ),
            ("^あれ$", "あれ", vec![Some(21), Some(0), None, None]),
            ("^がっこう$", "がっこう", vec![Some(1), None]),
            ("^xyzzlkj$", "", vec![]),
        ];
        for (pattern, text, expected_commons) in cases {
            let rows = find_word_kana_pattern(&ctx, pattern).await.unwrap();
            assert!(
                rows.iter().all(|row| row.text == *text),
                "pattern={pattern:?}: every row text should be {text:?}"
            );
            let commons: Vec<Option<i32>> = rows.iter().map(|row| row.common).collect();
            assert_eq!(&commons, expected_commons, "pattern={pattern:?}");
        }
    }

    /// REPL fixtures (.103), 2026-05-25 — single-row patterns pin the
    /// exact selected row (regex select + identity sort of one element).
    #[tokio::test]
    async fn single_row_patterns() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, i32, i32, Option<i32>)] = &[
            // pattern, seq, id, common
            ("^ねこ$", 1467640, 54168, Some(7)),
            ("^きそうてんがい$", 1219430, 28651, Some(26)),
        ];
        for (pattern, seq, id, common) in cases {
            let rows = find_word_kana_pattern(&ctx, pattern).await.unwrap();
            assert_eq!(rows.len(), 1, "pattern={pattern:?}");
            assert_eq!(rows[0].seq, *seq, "pattern={pattern:?}");
            assert_eq!(rows[0].id, *id, "pattern={pattern:?}");
            assert_eq!(rows[0].common, *common, "pattern={pattern:?}");
        }
    }
}

#[cfg(test)]
mod test_find_kanji_for_pattern {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::find-kanji-for-pattern`), 2026-05-25.
    /// `^つくえ$` is a single reading with a single kanji; `^がっこう$`
    /// pins kanji order by distinct `common` (学校 = 1 before 楽校 = null);
    /// `^あれ$` exercises the `when k` skip (its fourth `あれ` row has no
    /// kanji) plus the kana dedup (four rows collapse to one `あれ`);
    /// `^xyzzlkj$` returns two empty lists.
    #[tokio::test]
    async fn find_kanji_for_pattern_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, Vec<&str>, Vec<&str>)] = &[
            ("^つくえ$", vec!["机"], vec!["つくえ"]),
            ("^がっこう$", vec!["学校", "楽校"], vec!["がっこう"]),
            ("^あれ$", vec!["荒れ", "彼", "有れ"], vec!["あれ"]),
            ("^xyzzlkj$", vec![], vec![]),
        ];
        for (pattern, expected_kanji, expected_kana) in cases {
            let (kanji, kana) = find_kanji_for_pattern(&ctx, pattern).await.unwrap();
            assert_eq!(&kanji, expected_kanji, "pattern={pattern:?} (kanji)");
            assert_eq!(&kana, expected_kana, "pattern={pattern:?} (kana)");
        }
    }

    #[test]
    fn dedup_keeps_first_occurrence() {
        // `:from-end t` keeps the first occurrence and preserves order.
        let input = vec![
            "橋".to_string(),
            "端".to_string(),
            "橋".to_string(),
            "箸".to_string(),
            "端".to_string(),
        ];
        assert_eq!(
            remove_duplicates_from_end(input),
            vec!["橋".to_string(), "端".to_string(), "箸".to_string()]
        );
    }
}

#[cfg(test)]
mod test_exists_reading {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL (.103, `ichiran/dict::exists-reading`) + local DB, 2026-05-24:
    /// 政府 seq 1376070 has kana-text row "せいふ"
    /// (`(exists-reading 1376070 "せいふ")` -> `((1376070))`); reading
    /// "ありえない" is absent for that seq (-> `NIL`); 猫 seq 1467640
    /// has kana-text row "ねこ".
    #[tokio::test]
    async fn reading_present_and_absent() {
        let ctx = ctx().await;
        assert_eq!(
            exists_reading(&ctx, 1376070, "せいふ").await.unwrap(),
            vec![1376070]
        );
        assert!(exists_reading(&ctx, 1376070, "ありえない")
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            exists_reading(&ctx, 1467640, "ねこ").await.unwrap(),
            vec![1467640]
        );
        // reading belongs to a different entry -> no row for this seq
        assert!(exists_reading(&ctx, 1467640, "せいふ")
            .await
            .unwrap()
            .is_empty());
    }
}

#[cfg(test)]
mod test_get_candidates {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL probe pinned 2026-05-22 covers both branches and the no-row
    //! cases:
    //!   `(get-candidates "する" nil)` → `NIL` (kana branch, no root-p kana-only row).
    //!   `(get-candidates "漢字" "かんじ")` → `(1213170)` (kanji branch).
    //!   `(get-candidates "テスト" nil)` → `(1079760)` (kana branch hit).
    //!   `(get-candidates "ジャバスクリプトーー" nil)` → `NIL` (kana, no match).
    //!   `(get-candidates "漢字" "ZZZZZZZZ")` → `NIL` (kanji, bogus reading).
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn kana_branch_no_root_kana_only_entry() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "する", None).await.unwrap();
        assert!(out.is_empty(), "expected NIL, got {:?}", out);
    }

    #[tokio::test]
    async fn kanji_branch_with_reading() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "漢字", Some("かんじ")).await.unwrap();
        assert_eq!(out, vec![1213170]);
    }

    #[tokio::test]
    async fn kana_branch_pure_katakana_hit() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "テスト", None).await.unwrap();
        assert_eq!(out, vec![1079760]);
    }

    #[tokio::test]
    async fn kana_branch_unknown_kana_returns_empty() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "ジャバスクリプトーー", None)
            .await
            .unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn kanji_branch_bogus_reading_returns_empty() {
        let ctx = ctx_from_env().await;
        let out = get_candidates(&ctx, "漢字", Some("ZZZZZZZZ"))
            .await
            .unwrap();
        assert!(out.is_empty());
    }
}

#[cfg(test)]
mod test_get_non_arch_posi {
    use super::*;
    use crate::conn::kani_context::KaniranContext;

    // All assertions REPL-pinned against upstream ichiran. Each test
    // sorts the returned Vec before comparing because the upstream
    // Lisp `(:select … :distinct …)` does not impose an ORDER BY,
    // and Postgres is free to return distinct rows in any order.
    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    #[tokio::test]
    async fn taberu_single_seq() {
        // (get-non-arch-posi '(1357400)) → ("v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400]).await.expect("query");
        assert_eq!(sorted(got), vec!["v5m".to_string(), "vt".to_string()]);
    }

    #[tokio::test]
    async fn no_particle_seq() {
        // (get-non-arch-posi '(2089020)) → ("aux-v" "cop" "cop-da")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2089020]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec!["aux-v".to_string(), "cop".to_string(), "cop-da".to_string(),]
        );
    }

    #[tokio::test]
    async fn dummy_seq_1000220() {
        // (get-non-arch-posi '(1000220)) → ("adj-na")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1000220]).await.expect("query");
        assert_eq!(sorted(got), vec!["adj-na".to_string()]);
    }

    #[tokio::test]
    async fn hon_noun_seq() {
        // (get-non-arch-posi '(1522150)) → ("ctr" "n" "pref")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1522150]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec!["ctr".to_string(), "n".to_string(), "pref".to_string()]
        );
    }

    #[tokio::test]
    async fn counter_seq_1325880() {
        // (get-non-arch-posi '(1325880)) → ("n")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1325880]).await.expect("query");
        assert_eq!(sorted(got), vec!["n".to_string()]);
    }

    #[tokio::test]
    async fn two_seqs_union() {
        // (get-non-arch-posi '(1357400 2089020))
        //   → ("aux-v" "cop" "cop-da" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020])
            .await
            .expect("query");
        assert_eq!(
            sorted(got),
            vec![
                "aux-v".to_string(),
                "cop".to_string(),
                "cop-da".to_string(),
                "v5m".to_string(),
                "vt".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn zo_particle_seq() {
        // (get-non-arch-posi '(2029110)) → ("int" "prt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2029110]).await.expect("query");
        assert_eq!(sorted(got), vec!["int".to_string(), "prt".to_string()]);
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // (get-non-arch-posi '(99999999)) → NIL
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[99999999]).await.expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[tokio::test]
    async fn empty_seq_set_returns_empty() {
        // (get-non-arch-posi nil) → NIL.
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[]).await.expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[tokio::test]
    async fn many_seqs_union() {
        // (get-non-arch-posi '(1357400 2089020 1522150 1000220))
        //   → ("adj-na" "aux-v" "cop" "cop-da" "ctr" "n" "pref" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020, 1522150, 1000220])
            .await
            .expect("query");
        assert_eq!(
            sorted(got),
            vec![
                "adj-na".to_string(),
                "aux-v".to_string(),
                "cop".to_string(),
                "cop-da".to_string(),
                "ctr".to_string(),
                "n".to_string(),
                "pref".to_string(),
                "v5m".to_string(),
                "vt".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn taberu_with_conj_root() {
        // (get-non-arch-posi (list 1357400 2027820)) → ("exp" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2027820])
            .await
            .expect("query");
        assert_eq!(
            sorted(got),
            vec!["exp".to_string(), "v5m".to_string(), "vt".to_string()]
        );
    }
}

#[cfg(test)]
mod test_word_readings {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL fixtures (.103, ichiran/dict:word-readings), 2026-05-25.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn word_readings_fixtures() {
        let ctx = ctx_from_env().await;
        // (word, readings, romanizations). Cases cover:
        // - kana branch (word is itself in kana-text): word returned verbatim;
        // - kanji branch (ORDER BY id over the kana spellings): single & multi;
        // - katakana kana-branch input; macron long vowels; empty-on-both.
        let cases: &[(&str, &[&str], &[&str])] = &[
            // kanji branch, multiple kana readings ordered by id.
            (
                "猫",
                &["ねこ", "ネコ", "ねこま"],
                &["neko", "neko", "nekoma"],
            ),
            // kana branch — word is in kana-text, returned as-is.
            ("ねこ", &["ねこ"], &["neko"]),
            // kana branch, katakana surface form (long-vowel bar).
            ("コーヒー", &["コーヒー"], &["kohi"]),
            // kana branch, hiragana with macron long vowel.
            ("ありがとう", &["ありがとう"], &["arigatō"]),
            // kanji branch, single reading.
            ("図書館", &["としょかん"], &["toshokan"]),
            ("東京", &["とうきょう"], &["tōkyō"]),
            ("牛乳", &["ぎゅうにゅう"], &["gyūnyū"]),
            // mixed kanji+kana surface; not in kana-text, one kanji reading.
            ("食べる", &["たべる"], &["taberu"]),
            // not present in either table → empty kanji-seq → empty IN set.
            ("ヌルポポポ", &[], &[]),
        ];
        for (word, exp_readings, exp_rom) in cases {
            let (readings, romanizations) = word_readings(&ctx, word).await.unwrap();
            assert_eq!(&readings, exp_readings, "readings for {word}");
            assert_eq!(&romanizations, exp_rom, "romanizations for {word}");
        }
    }
}
