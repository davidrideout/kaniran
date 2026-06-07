use crate::characters::char_class::{test_word, CharClass};
use crate::conn::kani_context::KaniranContext;
use crate::dict::errata::get_all_readings;
use crate::dict::load::conj_rules::{
    get_conj_rules, ConjugationRule, DO_NOT_CONJUGATE, DO_NOT_CONJUGATE_SEQ, POS_WITH_CONJ_RULES,
    SECONDARY_CONJUGATION_TYPES, SECONDARY_CONJUGATION_TYPES_FROM,
};
use crate::dict::load::jmdict::next_seq;
use crate::dict::load::pos::{get_pos, get_pos_index};
use sqlx::Row;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Port of `ichiran/dict:construct-conjugation` (`dict-load.lisp:284`).
///
/// Assemble a conjugated reading from a dictionary `word` and a
/// `ConjugationRule`: drop `stem` trailing characters (one extra when
/// the applicable euphonic fragment is non-empty), then append the
/// euphonic fragment (`euphr` when the last two characters are kana,
/// `euphk` otherwise) and `okuri`. Offsets are character-based.
pub fn construct_conjugation(word: &str, rule: &ConjugationRule) -> String {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    // (subseq word (max 0 (- (length word) 2))) — last two characters
    let last_two: String = chars[len.saturating_sub(2)..].iter().collect();
    let iskana = test_word(&last_two, CharClass::Kana);
    let euphr = &rule.euphr;
    let euphk = &rule.euphk;
    let stem = rule.stem
        + if (iskana && !euphr.is_empty()) || (!iskana && !euphk.is_empty()) {
            1
        } else {
            0
        };
    let mut result: String = chars[..len - stem as usize].iter().collect();
    result.push_str(if iskana { euphr } else { euphk });
    result.push_str(&rule.okuri);
    result
}

/// Port of `ichiran/dict:conjugate-word` (`dict-load.lisp:296`).
///
/// Returns the list of (rule, conjugated-form) pairs produced by
/// applying every conjugation rule registered for `pos` to `word`.
pub fn conjugate_word(word: &str, pos: &str) -> Vec<(ConjugationRule, String)> {
    let pos_id = match get_pos_index(pos) {
        Some(id) => id,
        // dict-load.lisp:297 — get-pos-index returns nil for unknown pos;
        // (get-conj-rules nil) returns nil; the loop collects nothing.
        None => return Vec::new(),
    };
    let rules = get_conj_rules(pos_id);
    // dict-load.lisp:299-300 (loop for rule in rules collect (cons rule (construct-conjugation word rule)))
    rules
        .into_iter()
        .map(|rule| {
            let conjugated = construct_conjugation(word, &rule);
            (rule, conjugated)
        })
        .collect()
}

/// Port of `ichiran/dict:conjugate-entry-inner` (`dict-load.lisp:316`).
///
/// Build the conjugation matrix for one entry. For every pos tag on the
/// entry (or from `as_posi` when supplied) the function looks up the
/// conj rules, fetches the conjugatable kanji/kana readings, applies
/// [`construct_conjugation`], and slots each result into a 2×2 array
/// indexed by `[neg][fml]` under the `(pos-id, conj-id)` key.
/// One row in a `ConjMatrix` cell — the 5-element list
/// `(conj-text kanji-flag reading ord onum)` pushed at
/// `dict-load.lisp:338-341`. Example value:
/// `("食べた".to_string(), 1, "食べる".to_string(), 0, 1)` — past-plain
/// form of 食べる from its `ord=0` kanji reading, rule `onum=1`.
pub type ConjMatrixEntry = (String, i32, String, i32, i32);

/// `(pos-id, conj-id) → 2×2 array` where index `[neg][fml]` holds the
/// list of [`ConjMatrixEntry`] rows produced for that combination.
/// Mirrors the upstream `(make-hash-table :test 'equal)` /
/// `(make-array '(2 2) :initial-element nil)` shape at
/// `dict-load.lisp:319/337`.
pub type ConjMatrix = HashMap<(i32, i32), [[Vec<ConjMatrixEntry>; 2]; 2]>;

pub async fn conjugate_entry_inner(
    ctx: &KaniranContext,
    seq: i32,
    conj_types: Option<&[i32]>,
    as_posi: Option<&[String]>,
) -> Result<ConjMatrix, sqlx::Error> {
    // dict-load.lisp:317-318 — (or as-posi (query (:select 'text :distinct ...)))
    let posi: Vec<String> = match as_posi {
        Some(p) => p.to_vec(),
        None => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT DISTINCT text FROM sense_prop WHERE tag = 'pos' AND seq = $1",
            )
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
            rows.into_iter().map(|(t,)| t).collect()
        }
    };

    // dict-load.lisp:319 — (make-hash-table :test 'equal)
    let mut conj_matrix: ConjMatrix = HashMap::new();

    for pos in &posi {
        // dict-load.lisp:321 — (get-pos-index pos)
        let pos_id = match get_pos_index(pos) {
            Some(id) => id,
            None => continue,
        };
        // dict-load.lisp:322 — (get-conj-rules pos-id)
        let rules = get_conj_rules(pos_id);
        // dict-load.lisp:323 — (and rules (not (member pos *do-not-conjugate* :test 'equal)))
        if rules.is_empty() || DO_NOT_CONJUGATE.contains(&pos.as_str()) {
            continue;
        }
        // dict-load.lisp:325-328 — (:union (:select 'text 'ord 1 :from 'kanji-text ...) (:select 'text 'ord 0 :from 'kana-text ...))
        let readings: Vec<(String, i32, i32)> = sqlx::query_as(
            "SELECT text, ord, 1 AS kanji_flag FROM kanji_text \
             WHERE seq = $1 AND conjugate_p \
             UNION \
             SELECT text, ord, 0 AS kanji_flag FROM kana_text \
             WHERE seq = $1 AND conjugate_p",
        )
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?;

        for (reading, ord, kanji_flag) in &readings {
            for rule in &rules {
                let conj_id = rule.conj;
                // dict-load.lisp:331-332 — (or (not conj-types) (member conj-id conj-types))
                if let Some(types) = conj_types {
                    if !types.contains(&conj_id) {
                        continue;
                    }
                }
                let key = (pos_id, conj_id);
                let conj_text = construct_conjugation(reading, rule);
                // dict-load.lisp:335-337 — (unless (gethash key conj-matrix) (setf … (make-array '(2 2) :initial-element nil)))
                let cell = conj_matrix.entry(key).or_default();
                // dict-load.lisp:338-341 — (push (list conj-text kanji-flag reading ord (cr-onum rule)) (aref … (if (cr-neg rule) 1 0) (if (cr-fml rule) 1 0)))
                let neg_idx = if rule.neg { 1 } else { 0 };
                let fml_idx = if rule.fml { 1 } else { 0 };
                cell[neg_idx][fml_idx].insert(
                    0,
                    (conj_text, *kanji_flag, reading.clone(), *ord, rule.onum),
                );
            }
        }
    }
    Ok(conj_matrix)
}

/// Port of `ichiran/dict:conjugate-entry-outer` (`dict-load.lisp:344`).
///
/// Drives [`insert_conjugation`] over every cell of the conjugation
/// matrix built by [`conjugate_entry_inner`], writing the new
/// conjugated-reading rows for an entry.
pub async fn conjugate_entry_outer(
    ctx: &KaniranContext,
    seq_star: i32,
    via: Option<i32>,
    conj_types: Option<&[i32]>,
    as_posi: Option<&[String]>,
) -> Result<(), sqlx::Error> {
    // dict-load.lisp:345 — (or via seq*)
    let seq = via.unwrap_or(seq_star);
    // dict-load.lisp:346 — (conjugate-entry-inner seq :conj-types conj-types :as-posi as-posi)
    let conj_matrix = conjugate_entry_inner(ctx, seq, conj_types, as_posi).await?;
    // dict-load.lisp:347 — (get-all-readings seq)
    let original_readings = get_all_readings(ctx, seq).await?;
    // dict-load.lisp:348 — (next-seq)
    let mut next_seq_val = next_seq(ctx).await?;

    // dict-load.lisp:349 — iterate hash-key (pos-id conj-id) / hash-value matrix.
    // Sort by (pos_id, conj_id) so iteration is deterministic. Order matters
    // downstream
    let mut entries: Vec<_> = conj_matrix.iter().collect();
    entries.sort_by_key(|((pos_id, conj_id), _)| (*pos_id, *conj_id));
    for ((pos_id, conj_id), matrix) in entries {
        // dict-load.lisp:350-351 — ignore-neg / ignore-fml flags
        let ignore_neg = matrix[1][0].is_empty() && matrix[1][1].is_empty();
        let ignore_fml = matrix[0][1].is_empty() && matrix[1][1].is_empty();
        // dict-load.lisp:352 — (get-pos pos-id)
        let pos = get_pos(*pos_id)
            .expect("conjugate-entry-outer: pos-id in conj-matrix not in *pos-by-index*");
        // dict-load.lisp:353-365 — loop for ii from 0 below 4
        for ii in 0..4 {
            let neg_flag = ii >= 2;
            let fml_flag = ii % 2 != 0;
            // dict-load.lisp:357-358 — (row-major-aref matrix ii)
            let cell = &matrix[ii / 2][ii % 2];
            // dict-load.lisp:356-358 — (remove-if (lambda (item) (member (car item) original-readings :test 'equal)))
            let readings: Vec<_> = cell
                .iter()
                .filter(|item| !original_readings.contains(&item.0))
                .cloned()
                .collect();
            if readings.is_empty() {
                continue;
            }
            // dict-load.lisp:360-364 — insert-conjugation … :neg (if ignore-neg :null neg) :fml (if ignore-fml :null fml)
            let inserted = insert_conjugation(
                ctx,
                &readings,
                next_seq_val,
                seq_star,
                pos,
                *conj_id,
                if ignore_neg { None } else { Some(neg_flag) },
                if ignore_fml { None } else { Some(fml_flag) },
                via,
            )
            .await?;
            if inserted {
                next_seq_val += 1;
            }
        }
    }
    Ok(())
}

/// Port of `ichiran/dict:lex-compare` (`dict-load.lisp:367`).
///
/// Returns a lexicographic comparator (a closure) parameterised on the
/// element-level `predicate`. Walks two equal-length sequences in
/// lockstep; the first pair where `predicate(e1, e2)` is true makes the
/// comparator return `true`, the first pair where `predicate(e2, e1)`
/// is true makes it return `false`. If neither holds for any pair, the
/// comparator returns `false`. Mismatched lengths walk only the shared
/// prefix and then return `false`.
pub fn lex_compare<T, P>(predicate: P) -> impl Fn(&[T], &[T]) -> bool
where
    P: Fn(&T, &T) -> bool,
{
    move |seq1, seq2| {
        // dict-load.lisp:371 (map nil (lambda (e1 e2) …) seq1 seq2)
        for (e1, e2) in seq1.iter().zip(seq2.iter()) {
            if predicate(e1, e2) {
                return true;
            }
            if predicate(e2, e1) {
                return false;
            }
        }
        // dict-load.lisp:370 (block nil …) — falls off `map nil` to nil.
        false
    }
}

/// Port of `ichiran/dict:insert-conjugation` (`dict-load.lisp:377`).
///
/// For one (pos-id, conj-id, neg, fml) cell of the conjugation matrix,
/// resolve an existing or freshly-minted target entry, then upsert the
/// corresponding `conjugation` / `conj_prop` / `conj_source_reading`
/// rows. Returns `true` iff a brand-new target entry was created.
pub async fn insert_conjugation(
    ctx: &KaniranContext,
    readings: &[ConjMatrixEntry],
    seq: i32,
    from: i32,
    pos: &str,
    conj_type: i32,
    neg: Option<bool>,
    fml: Option<bool>,
    via: Option<i32>,
) -> Result<bool, sqlx::Error> {
    // dict-load.lisp:379 — (sort readings (lex-compare #'<) :key #'cdddr)
    // cdddr of (conj-text kanji-flag reading ord onum) = (ord onum)
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let mut sorted: Vec<ConjMatrixEntry> = readings.to_vec();
    sorted.sort_by(|a, b| {
        let ka = [a.3, a.4];
        let kb = [b.3, b.4];
        if cmp(&ka, &kb) {
            Ordering::Less
        } else if cmp(&kb, &ka) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });

    // dict-load.lisp:380-382 — collect source-readings, kanji-readings, kana-readings
    let mut source_readings: Vec<(String, String)> = Vec::new();
    let mut kanji_readings_raw: Vec<String> = Vec::new();
    let mut kana_readings_raw: Vec<String> = Vec::new();
    for (reading, kanji_flag, orig_reading, _ord, _onum) in &sorted {
        source_readings.push((reading.clone(), orig_reading.clone()));
        if *kanji_flag == 1 {
            kanji_readings_raw.push(reading.clone());
        } else {
            kana_readings_raw.push(reading.clone());
        }
    }
    // dict-load.lisp:384 — (unless kana-readings (return nil))
    if kana_readings_raw.is_empty() {
        return Ok(false);
    }

    // dict-load.lisp:385-386 — (remove-duplicates … :test 'equal)
    // CL default keeps the last occurrence; mirror by reversing, keeping
    // first-seen, then reversing back.
    let kanji_readings = dedupe_keep_last(kanji_readings_raw);
    let kana_readings = dedupe_keep_last(kana_readings_raw);

    // dict-load.lisp:387-408 — seq-candidates (sort … '<)
    let seq_candidates: Vec<i32> = if !kanji_readings.is_empty() {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT seq FROM kanji_text WHERE text = ANY($1) \
             GROUP BY seq HAVING COUNT(id) = $2 \
             INTERSECT \
             SELECT seq FROM kana_text WHERE text = ANY($3) \
             GROUP BY seq HAVING COUNT(id) = $4 \
             ORDER BY seq",
        )
        .bind(&kanji_readings)
        .bind(kanji_readings.len() as i64)
        .bind(&kana_readings)
        .bind(kana_readings.len() as i64)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|(s,)| s).collect()
    } else {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT r.seq FROM kana_text r \
             LEFT JOIN kanji_text k ON r.seq = k.seq \
             WHERE k.text IS NULL AND r.text = ANY($1) \
             GROUP BY r.seq HAVING COUNT(r.id) = $2 \
             ORDER BY r.seq",
        )
        .bind(&kana_readings)
        .bind(kana_readings.len() as i64)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|(s,)| s).collect()
    };

    // dict-load.lisp:409-410 — (when (or (member from seq-candidates) (member via seq-candidates)) (return nil))
    if seq_candidates.contains(&from) || via.is_some_and(|v| seq_candidates.contains(&v)) {
        return Ok(false);
    }

    let mut seq = seq;
    // dict-load.lisp:411 — (if seq-candidates …)
    if let Some(first) = seq_candidates.first() {
        seq = *first;
    } else {
        // dict-load.lisp:414 — (make-dao 'entry :seq seq :content "")
        // entry initforms: root-p nil, n-kanji 0, n-kana 0, primary-nokanji nil.
        sqlx::query(
            "INSERT INTO entry (seq, content, root_p, n_kanji, n_kana, primary_nokanji) \
             VALUES ($1, '', FALSE, 0, 0, FALSE)",
        )
        .bind(seq)
        .execute(&ctx.pool)
        .await?;
        // dict-load.lisp:415 — (let ((conjugate-p (when (member conj-type *secondary-conjugation-types-from*) t))))
        let conjugate_p = SECONDARY_CONJUGATION_TYPES_FROM.contains(&conj_type);
        // dict-load.lisp:416-418 — kanji-text inserts
        // kanji-text initforms (dict.lisp:86): common-tags "", nokanji nil, best-kana :null.
        for (ord, kr) in kanji_readings.iter().enumerate() {
            sqlx::query(
                "INSERT INTO kanji_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kana) \
                 VALUES ($1, $2, $3, NULL, '', $4, FALSE, NULL)",
            )
            .bind(seq)
            .bind(kr)
            .bind(ord as i32)
            .bind(conjugate_p)
            .execute(&ctx.pool)
            .await?;
        }
        // dict-load.lisp:419-421 — kana-text inserts
        // kana-text initforms (dict.lisp:128): common-tags "", nokanji nil, best-kanji :null.
        for (ord, kr) in kana_readings.iter().enumerate() {
            sqlx::query(
                "INSERT INTO kana_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kanji) \
                 VALUES ($1, $2, $3, NULL, '', $4, FALSE, NULL)",
            )
            .bind(seq)
            .bind(kr)
            .bind(ord as i32)
            .bind(conjugate_p)
            .execute(&ctx.pool)
            .await?;
        }
    }

    // dict-load.lisp:423-425 — (if via … (:= 'via via) … (:is-null 'via))
    let old_conj_ids: Vec<i32> = if let Some(via_val) = via {
        let rows = sqlx::query(
            r#"SELECT id FROM conjugation WHERE "from" = $1 AND seq = $2 AND via = $3"#,
        )
        .bind(from)
        .bind(seq)
        .bind(via_val)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|r| r.get::<i32, _>("id")).collect()
    } else {
        let rows = sqlx::query(
            r#"SELECT id FROM conjugation WHERE "from" = $1 AND seq = $2 AND via IS NULL"#,
        )
        .bind(from)
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|r| r.get::<i32, _>("id")).collect()
    };
    // dict-load.lisp:426 — (or (car old-conj) (make-dao 'conjugation :seq seq :from from :via (or via :null)))
    let conj_id: i32 = if let Some(id) = old_conj_ids.first() {
        *id
    } else {
        sqlx::query_scalar(
            r#"INSERT INTO conjugation (seq, "from", via) VALUES ($1, $2, $3) RETURNING id"#,
        )
        .bind(seq)
        .bind(from)
        .bind(via)
        .fetch_one(&ctx.pool)
        .await?
    };

    // dict-load.lisp:428-434 — (unless (select-dao 'conj-prop …) (make-dao 'conj-prop …))
    // `:===` is null-safe equality; map to `IS NOT DISTINCT FROM`.
    let existing_prop: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM conj_prop \
         WHERE conj_id = $1 AND conj_type = $2 AND pos = $3 \
           AND neg IS NOT DISTINCT FROM $4 AND fml IS NOT DISTINCT FROM $5 \
         LIMIT 1",
    )
    .bind(conj_id)
    .bind(conj_type)
    .bind(pos)
    .bind(neg)
    .bind(fml)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing_prop.is_none() {
        sqlx::query(
            "INSERT INTO conj_prop (conj_id, conj_type, pos, neg, fml) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(conj_id)
        .bind(conj_type)
        .bind(pos)
        .bind(neg)
        .bind(fml)
        .execute(&ctx.pool)
        .await?;
    }

    // dict-load.lisp:436 — (let ((old-csr (when old-conj (query …)))))
    let old_csr: Vec<(String, String)> = if !old_conj_ids.is_empty() {
        sqlx::query_as("SELECT text, source_text FROM conj_source_reading WHERE conj_id = $1")
            .bind(conj_id)
            .fetch_all(&ctx.pool)
            .await?
    } else {
        Vec::new()
    };

    // dict-load.lisp:437 — (remove-duplicates (set-difference source-readings old-csr :test 'equal) :test 'equal)
    // set-difference preserves order of the first arg; remove-duplicates default keeps the
    // last occurrence.
    let source_readings: Vec<(String, String)> = {
        let old_set: HashSet<(String, String)> = old_csr.into_iter().collect();
        let diff: Vec<(String, String)> = source_readings
            .into_iter()
            .filter(|sr| !old_set.contains(sr))
            .collect();
        dedupe_keep_last(diff)
    };

    // dict-load.lisp:438-443 — per source-reading, INSERT if not already present.
    for (text, source_text) in &source_readings {
        let existing: Option<i32> = sqlx::query_scalar(
            "SELECT id FROM conj_source_reading \
             WHERE conj_id = $1 AND text = $2 AND source_text = $3 \
             LIMIT 1",
        )
        .bind(conj_id)
        .bind(text)
        .bind(source_text)
        .fetch_optional(&ctx.pool)
        .await?;
        if existing.is_none() {
            sqlx::query(
                "INSERT INTO conj_source_reading (conj_id, text, source_text) \
                 VALUES ($1, $2, $3)",
            )
            .bind(conj_id)
            .bind(text)
            .bind(source_text)
            .execute(&ctx.pool)
            .await?;
        }
    }

    // dict-load.lisp:445 — (return (not seq-candidates))
    Ok(seq_candidates.is_empty())
}

/// Mirror CL's `(remove-duplicates seq :test 'equal)` default behaviour:
/// preserves the **last** occurrence of each value (the earlier
/// duplicates are dropped).
fn dedupe_keep_last<T: std::hash::Hash + Eq + Clone>(v: Vec<T>) -> Vec<T> {
    let mut seen: HashSet<T> = HashSet::new();
    let mut result: Vec<T> = Vec::with_capacity(v.len());
    for item in v.into_iter().rev() {
        if seen.insert(item.clone()) {
            result.push(item);
        }
    }
    result.reverse();
    result
}

/// Port of `ichiran/dict:load-conjugations` (`dict-load.lisp:447`).
///
/// Walks every entry whose JMdict POS includes a conjugatable tag and
/// drives [`conjugate_entry_outer`] across it. Skip-list:
/// [`DO_NOT_CONJUGATE_SEQ`]. POS gate: [`POS_WITH_CONJ_RULES`].
pub async fn load_conjugations(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    // dict-load.lisp:449-453 (query (:select 'seq :distinct :from 'sense-prop
    //   :where (:and (:not (:in 'seq (:set *do-not-conjugate-seq*)))
    //                (:= 'tag "pos")
    //                (:in 'text (:set *pos-with-conj-rules*)))) :column)
    let seqs: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT seq FROM sense_prop \
         WHERE NOT (seq = ANY($1)) AND tag = 'pos' AND text = ANY($2)",
    )
    .bind(DO_NOT_CONJUGATE_SEQ)
    .bind(POS_WITH_CONJ_RULES)
    .fetch_all(&ctx.pool)
    .await?;
    // dict-load.lisp:454-457 (loop for cnt from 1 for seq in seqs ...)
    for (idx, seq) in seqs.iter().enumerate() {
        conjugate_entry_outer(ctx, *seq, None, None, None).await?;
        let cnt = idx + 1;
        if cnt % 500 == 0 {
            println!("{cnt} entries processed");
        }
    }
    Ok(())
}

/// Port of `ichiran/dict:load-secondary-conjugations` (`dict-load.lisp:460`).
///
/// Walks every primary conjugation tagged as a secondary type and drives
/// `conjugate_entry_outer` to build the second-order conjugations (`v5s`
/// posi for the causative-su source form, else `v1`); `from` restricts
/// the candidate set to the given source seqs.
// dict-errata.lisp:1239 (defconstant +conj-causative-su+ 53)
const CONJ_CAUSATIVE_SU: i32 = 53;

pub async fn load_secondary_conjugations(
    ctx: &KaniranContext,
    from: Option<&[i32]>,
) -> Result<(), sqlx::Error> {
    // dict-load.lisp:461-473 (sql-compile of the :select conj.from conj.seq conj-prop.conj-type ...)
    let mut sql = String::from(
        "SELECT DISTINCT ON (conj.\"from\", conj.seq) \
         conj.\"from\", conj.seq, conj_prop.conj_type \
         FROM conjugation conj \
         LEFT JOIN conj_prop ON conj.id = conj_prop.conj_id \
         WHERE ",
    );
    let mut conds: Vec<String> = Vec::new();
    if from.is_some() {
        conds.push("conj.\"from\" = ANY($2)".to_string());
    }
    conds.push("conj_prop.conj_type = ANY($1)".to_string());
    conds.push("NOT (conj_prop.pos = ANY(ARRAY['vs-i','vs-s']))".to_string());
    conds.push("conj.via IS NULL".to_string());
    conds.push("(NOT conj_prop.neg OR conj_prop.neg IS NULL)".to_string());
    conds.push("(NOT conj_prop.fml OR conj_prop.fml IS NULL)".to_string());
    sql.push_str(&conds.join(" AND "));

    let to_conj: Vec<(i32, i32, i32)> = {
        let q = sqlx::query_as(&sql).bind(SECONDARY_CONJUGATION_TYPES_FROM);
        let q = if let Some(from_seqs) = from {
            q.bind(from_seqs)
        } else {
            q
        };
        q.fetch_all(&ctx.pool).await?
    };

    // dict-load.lisp:475-480 (loop for cnt from 1 for (seq-from seq conj-type) in to-conj do ...)
    for (idx, (seq_from, seq, conj_type)) in to_conj.iter().enumerate() {
        let as_posi: [String; 1] = [if *conj_type == CONJ_CAUSATIVE_SU {
            "v5s".to_string()
        } else {
            "v1".to_string()
        }];
        conjugate_entry_outer(
            ctx,
            *seq_from,
            Some(*seq),
            Some(SECONDARY_CONJUGATION_TYPES),
            Some(&as_posi),
        )
        .await?;
        let cnt = idx + 1;
        if cnt % 1000 == 0 {
            println!("{cnt} entries processed");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
