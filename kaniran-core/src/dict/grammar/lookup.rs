use crate::conn::kani_backend::KaniBackend;
use crate::characters::char_class::{test_word, CharClass};
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj::ConjData;
use crate::dict::dao::Conjugation;
use crate::dict::counters::methods::seq;
use crate::dict::errata::{skip_by_conj_data, test_conj_prop, WEAK_CONJ_FORMS};
use crate::dict::readings::FindWordRows;
use crate::dict::text_classes::{find_word_as_hiragana, HiraganaFinder};
use crate::dict::path::find_word_full;
use crate::dict::conj::{get_conj_data, FromOrConjIds};
use crate::dict::grammar::suffix::constants::suffix_class;
use crate::dict::dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::dao::KanjiText;
use crate::dict::load::conjugate::lex_compare;
use crate::dict::text_classes::ProxyText;
use crate::dict::accessors::set_word_conjugations;
use crate::dict::dao::WordConjugations;
use crate::dict::accessors::word_conj_data;
use crate::dict::accessors::word_conjugations;
use crate::dict::word_info::WordInfoSeq;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Port of `ichiran/dict:get-kana-forms-conj-data-filter` (`dict-grammar.lisp:10`).
///
/// Short-circuits to empty if [`skip_by_conj_data`] applies; otherwise
/// collects `conj_id`s of props that are NOT in [`WEAK_CONJ_FORMS`].
pub fn get_kana_forms_conj_data_filter(conj_data: &[ConjData]) -> Vec<i32> {
    if skip_by_conj_data(conj_data) {
        return Vec::new();
    }
    conj_data
        .iter()
        .filter_map(|cd| {
            let prop = cd.prop.as_ref()?;
            if test_conj_prop(prop, WEAK_CONJ_FORMS) {
                None
            } else {
                Some(prop.conj_id)
            }
        })
        .collect()
}

/// Transliteration of `ichiran/dict:get-kana-forms*` (`dict-grammar.lisp:17`).
///
/// Loads every kana-text tied to `seq` — directly or via
/// `conjugation.from = seq` — and tags each row's runtime
/// `state.conjugations`: original-entry rows get `Root`, derived rows
/// get `Ids(...)` from [`get_kana_forms_conj_data_filter`]. Derived
/// rows with an empty filter are dropped.
///
/// [`get_kana_forms_conj_data_filter`]: crate::dict::grammar::lookup::get_kana_forms_conj_data_filter
pub async fn get_kana_forms_star_(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<KanaText>, sqlx::Error> {
    let kts: Vec<KanaText> = ctx.store.kana_forms_rows(seq).await?;

    let mut out: Vec<KanaText> = Vec::with_capacity(kts.len());
    for mut kt in kts {
        if kt.seq == seq {
            kt.state.conjugations = Some(WordConjugations::Root);
            out.push(kt);
        } else {
            let cd = get_conj_data(ctx, kt.seq, FromOrConjIds::From(seq), &[]).await?;
            let conj_ids = get_kana_forms_conj_data_filter(&cd);
            if !conj_ids.is_empty() {
                kt.state.conjugations = Some(WordConjugations::Ids(conj_ids));
                out.push(kt);
            }
        }
    }
    Ok(out)
}

/// Transliteration of `ichiran/dict:get-kana-forms` (`dict-grammar.lisp:32`).
///
/// Thin wrapper: returns [`get_kana_forms_star_`]; emits a stderr
/// warning when the result is empty.
///
/// [`get_kana_forms_star_`]: crate::dict::grammar::lookup::get_kana_forms_star_
pub async fn get_kana_forms(ctx: &KaniranContext, seq: i32) -> Result<Vec<KanaText>, sqlx::Error> {
    let result = get_kana_forms_star_(ctx, seq).await?;
    if result.is_empty() {
        eprintln!("kaniran: No kana forms found for: {seq}");
    }
    Ok(result)
}

/// Transliteration of `ichiran/dict:get-kana-form` (`dict-grammar.lisp:38`).
///
/// Looks up the first kana_text row by `(text, seq)` pair. If a `conj`
/// value is supplied, mutates the loaded row's runtime-only
/// conjugations slot before returning it.
pub async fn get_kana_form(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
    conj: Option<WordConjugations>,
) -> Result<Option<KanaText>, sqlx::Error> {
    let row = ctx
        .store
        .kana_texts_by_text_and_seq(text, seq)
        .await?
        .into_iter()
        .next();
    Ok(row.map(|mut r| {
        if let Some(c) = conj {
            r.state.conjugations = Some(c);
        }
        r
    }))
}

/// Port of `ichiran/dict:find-word-with-conj-prop` (`dict-grammar.lisp:44`).
///
/// For each word from [`find_word_full`], read its conjugation data
/// through [`word_conj_data`], filter with the caller's predicate, and
/// collect the word — setting its `word-conjugations` slot to the
/// survivors' conj-ids — when either the filtered list is non-empty, or
/// the original conj-data was empty and `allow_root` is set.
///
/// [`find_word_full`]: crate::dict::path::find_word_full
/// [`word_conj_data`]: crate::dict::accessors::word_conj_data
pub async fn find_word_with_conj_prop<F>(
    ctx: &KaniranContext,
    wordstr: &str,
    mut filter_fn: F,
    allow_root: bool,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error>
where
    F: FnMut(&ConjData) -> bool,
{
    let words = find_word_full(ctx, wordstr, false, None).await?;
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    for mut word in words {
        // dict-grammar.lisp:45 (word-conj-data word)
        let conj_data = word_conj_data(ctx, &word).await?;
        // dict-grammar.lisp:46 (remove-if-not filter-fn conj-data)
        let filtered: Vec<&ConjData> = conj_data.iter().filter(|cd| filter_fn(cd)).collect();
        // dict-grammar.lisp:47 (mapcar (lambda (cdata) (conj-id (conj-data-prop cdata))) ...)
        let conj_ids: Vec<i32> = filtered
            .iter()
            .filter_map(|cd| cd.prop.as_ref().map(|p| p.conj_id))
            .collect();
        // dict-grammar.lisp:48 (when (or conj-data-filtered (and (null conj-data) allow-root))
        let allow_root_path = conj_data.is_empty() && allow_root;
        if !filtered.is_empty() || allow_root_path {
            // dict-grammar.lisp:49 (setf (word-conjugations word) conj-ids)
            // conj_ids may be empty (mapcar over nil = nil) — preserve
            // the upstream "nil-set" by passing None.
            let new_value = if conj_ids.is_empty() {
                None
            } else {
                Some(WordConjugations::Ids(conj_ids))
            };
            set_word_conjugations(&mut word, new_value);
            out.push(word);
        }
    }
    Ok(out)
}

/// Port of `ichiran/dict:find-word-with-conj-type` (`dict-grammar.lisp:53`).
///
/// Delegates to [`find_word_with_conj_prop`] with a filter that admits
/// any conj-data whose prop's `conj-type` is in the caller-supplied
/// integer set.
///
/// [`find_word_with_conj_prop`]: crate::dict::grammar::lookup::find_word_with_conj_prop
pub async fn find_word_with_conj_type(
    ctx: &KaniranContext,
    word: &str,
    conj_types: &[i32],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    find_word_with_conj_prop(
        ctx,
        word,
        // dict-grammar.lisp:54-56 — (lambda (cdata) (member (conj-type (conj-data-prop cdata)) conj-types))
        // Lisp `(member x nil)` is nil — closing over an empty slice
        // returns false for every cdata, mirroring (member … '()).
        |cd| {
            cd.prop
                .as_ref()
                .is_some_and(|p| conj_types.contains(&p.conj_type))
        },
        false,
    )
    .await
}

/// Port of `ichiran/dict:pair-words-by-conj` (`dict-grammar.lisp:58`).
///
/// Group N word-lists into buckets where words share the same
/// conjugation-source signature, placing each word at its source-list
/// index inside the bucket. The signature is the sorted list of
/// `(seq-from, via-or-0)` pairs from the word's conjugations; bucket
/// cell `[idx]` holds the word from input list `idx` (or `None`).
pub async fn pair_words_by_conj(
    ctx: &KaniranContext,
    word_groups: &[Vec<KaniWordDispatchEnum>],
) -> Result<Vec<Vec<Option<KaniWordDispatchEnum>>>, sqlx::Error> {
    // dict-grammar.lisp:64 — (sort … (lex-compare '<))
    let pair_cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let mut bag: HashMap<Vec<i32>, Vec<Option<KaniWordDispatchEnum>>> = HashMap::new();

    // dict-grammar.lisp:65-72 — outer loop walks word-groups with idx.
    for (idx, wg) in word_groups.iter().enumerate() {
        for word in wg {
            let key = compute_key(ctx, word, &pair_cmp).await?;
            // dict-grammar.lisp:70 — (or (gethash key bag) (loop … collect nil)).
            let arr = bag
                .entry(key)
                .or_insert_with(|| vec![None; word_groups.len()]);
            // dict-grammar.lisp:71-72 — (setf (elt arr idx) word).
            arr[idx] = Some(word.clone());
        }
    }
    // dict-grammar.lisp:73 — (alexandria:hash-table-values bag).
    Ok(bag.into_values().collect())
}

async fn compute_key(
    ctx: &KaniranContext,
    word: &KaniWordDispatchEnum,
    pair_cmp: &impl Fn(&[i32], &[i32]) -> bool,
) -> Result<Vec<i32>, sqlx::Error> {
    // dict-grammar.lisp:60-63 — (mapcar (lambda (conj-id) …) (word-conjugations word)).
    // CL `(mapcar f nil)` → nil; `(mapcar f :root)` would TYPE-ERROR.
    let conj_ids: Vec<i32> = match word_conjugations(word) {
        Some(WordConjugations::Ids(ids)) => ids,
        None => Vec::new(),
        Some(WordConjugations::Root) => {
            unreachable!(
                "pair-words-by-conj received a :root-tagged word; upstream \
                 (mapcar f :root) signals a TYPE-ERROR and no producer in the \
                 call graph (find-word-with-conj-prop) sets word-conjugations \
                 to :root"
            )
        }
    };
    let mut pairs: Vec<[i32; 2]> = Vec::with_capacity(conj_ids.len());
    for cid in &conj_ids {
        // dict-grammar.lisp:61 — (get-dao 'conjugation conj-id)
        let conj: Conjugation = ctx.store.conj_by_id(*cid).await?;
        // dict-grammar.lisp:62 — (let ((via (seq-via conj))) (if (eql via :null) 0 via)).
        let via = conj.seq_via.unwrap_or(0);
        pairs.push([conj.seq_from, via]);
    }
    // dict-grammar.lisp:64 — sort the pair list with the lex-compare comparator.
    pairs.sort_by(|a, b| {
        if pair_cmp(a, b) {
            Ordering::Less
        } else if pair_cmp(b, a) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
    // Hash key: flatten the sorted pair list. Inner length is fixed at 2,
    // so the flat sequence is a deterministic bijection of the nested form
    // and matches CL `equal` on the original list-of-2-element-lists.
    Ok(pairs.into_iter().flatten().collect())
}

/// Port of `ichiran/dict:find-word-seq` (`dict-grammar.lisp:75`).
///
/// Returns the rows of `kana_text` or `kanji_text` where `text = word`
/// and `seq ∈ seqs`. The table is chosen by
/// [`crate::characters::test_word`] against [`CharClass::Kana`] — kana
/// inputs hit `kana_text`, anything else hits `kanji_text`.
#[derive(Debug, Clone)]
pub enum WordSeqRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

impl WordSeqRows {
    /// `(car ...)` over the row vector, wrapping the row as a
    /// [`KaniWordDispatchEnum`] for downstream dispatchers. Empty
    /// vector yields `None`. Used by `*split-map*` entries that
    /// resolve a part via `(car (apply find-word-seq ...))`.
    pub(crate) fn first_word(self) -> Option<KaniWordDispatchEnum> {
        match self {
            Self::Kana(v) => v.into_iter().next().map(KaniWordDispatchEnum::Kana),
            Self::Kanji(v) => v.into_iter().next().map(KaniWordDispatchEnum::Kanji),
        }
    }

    /// First row's `seq`, used by the `("text" seq) part-seq` form
    /// of `def-simple-split` to compute the dynamic pseq via
    /// `(seq (car (find-word-conj-of "text" seq)))`. Empty vector
    /// yields `None`.
    pub(crate) fn first_seq(&self) -> Option<i32> {
        match self {
            Self::Kana(v) => v.first().map(|r| r.seq),
            Self::Kanji(v) => v.first().map(|r| r.seq),
        }
    }
}

pub async fn find_word_seq(
    ctx: &KaniranContext,
    word: &str,
    seqs: &[i32],
) -> Result<WordSeqRows, sqlx::Error> {
    if test_word(word, CharClass::Kana) {
        let rows = ctx.store.kana_texts_by_text_and_seq_any(word, seqs).await?;
        Ok(WordSeqRows::Kana(rows))
    } else {
        let rows = ctx
            .store
            .kanji_texts_by_text_and_seq_any(word, seqs)
            .await?;
        Ok(WordSeqRows::Kanji(rows))
    }
}

/// Port of `ichiran/dict:find-word-conj-of` (`dict-grammar.lisp:79`).
///
/// Returns kana_text or kanji_text rows for `word` matching either any
/// of `seqs` directly, or any seq joined to `seqs` via the
/// `conjugation.from` column. The two row sets are unioned by `id`
/// following SBCL's `union` ordering: `(reverse <longer list's
/// uniques>) ++ <shorter list>`, with no de-dup within either list.
pub async fn find_word_conj_of(
    ctx: &KaniranContext,
    word: &str,
    seqs: &[i32],
) -> Result<WordSeqRows, sqlx::Error> {
    let primary = find_word_seq(ctx, word, seqs).await?;
    if test_word(word, CharClass::Kana) {
        let conj_rows: Vec<KanaText> = ctx.store.kana_texts_conj_of(seqs, word).await?;
        let primary_rows = match primary {
            WordSeqRows::Kana(v) => v,
            WordSeqRows::Kanji(_) => unreachable!(
                "test_word dispatch must agree between find-word-seq and find-word-conj-of"
            ),
        };
        Ok(WordSeqRows::Kana(union_by_id(
            primary_rows,
            conj_rows,
            |r| r.id,
        )))
    } else {
        let conj_rows: Vec<KanjiText> = ctx.store.kanji_texts_conj_of(seqs, word).await?;
        let primary_rows = match primary {
            WordSeqRows::Kanji(v) => v,
            WordSeqRows::Kana(_) => unreachable!(
                "test_word dispatch must agree between find-word-seq and find-word-conj-of"
            ),
        };
        Ok(WordSeqRows::Kanji(union_by_id(
            primary_rows,
            conj_rows,
            |r| r.id,
        )))
    }
}

/// `(union list1 list2 :key id)` for SBCL semantics — NOT a generic
/// set union. SBCL picks the longer list (list1 wins length-tie),
/// starts a fresh result, copies the shorter list in, then walks the
/// longer list left-to-right `cons`-pushing each non-duplicate. The
/// final shape is `reverse(<longer's uniques>) ++ <shorter>`.
///
/// Empirically verified on SBCL 2.2.9:
/// - `(union '() '(1 2 3))`            → `(3 2 1)`
/// - `(union '(1 2 3) '())`            → `(3 2 1)`
/// - `(union '(1 2 3) '(4 5 6))`       → `(3 2 1 4 5 6)` (list1 wins tie)
/// - `(union '(4) '(1 2 3))`           → `(3 2 1 4)` (list2 longer)
/// - `(union '(1 2 3) '(2))`           → `(3 1 2)` (skip dup 2)
fn union_by_id<T>(list1: Vec<T>, list2: Vec<T>, id: impl Fn(&T) -> i32) -> Vec<T> {
    let (shorter, longer) = if list1.len() >= list2.len() {
        (list2, list1)
    } else {
        (list1, list2)
    };
    let shorter_keys: HashSet<i32> = shorter.iter().map(&id).collect();
    let mut uniques: Vec<T> = Vec::new();
    for elt in longer {
        if !shorter_keys.contains(&id(&elt)) {
            uniques.push(elt);
        }
    }
    uniques.reverse();
    uniques.extend(shorter);
    uniques
}

/// Port of `ichiran/dict:find-word-with-pos` (`dict-grammar.lisp:89`).
///
/// Returns kana_text or kanji_text rows whose `text = word` and whose
/// seq is the seq of a sense flagged with one of the given `pos` tags
/// in `sense_prop`. The table is chosen by
/// [`crate::characters::test_word`] against [`CharClass::Kana`] — kana
/// inputs hit `kana_text`, anything else hits `kanji_text`.
#[derive(Debug, Clone)]
pub enum WordWithPosRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

pub async fn find_word_with_pos(
    ctx: &KaniranContext,
    word: &str,
    posi: &[&str],
) -> Result<WordWithPosRows, sqlx::Error> {
    // s-sql `:in 'sp.text (:set posi)` expands to multiple `?` binds;
    // Postgres' `sp.text = ANY($2)` is the array-bound equivalent. The
    // sqlx Encode impl for `&[&str]` over Postgres requires owned
    // String elements, so allocate a Vec<String> for the bind (see
    // dict/get_conj_data.rs:67 for the same pattern).
    let posi_owned: Vec<String> = posi.iter().map(|s| (*s).to_string()).collect();
    if test_word(word, CharClass::Kana) {
        let rows = ctx.store.kana_texts_with_pos(word, &posi_owned).await?;
        Ok(WordWithPosRows::Kana(rows))
    } else {
        let rows = ctx.store.kanji_texts_with_pos(word, &posi_owned).await?;
        Ok(WordWithPosRows::Kanji(rows))
    }
}

/// Port of `ichiran/dict:or-as-hiragana` (`dict-grammar.lisp:95`).
///
/// Calls `fn_(word)`; if non-empty, returns those rows as
/// [`OrAsHiraganaRows::Direct`]. Otherwise re-enters through
/// [`find_word_as_hiragana`] with `fn_` as the finder, wrapping each
/// result as a [`ProxyText`] carrying `word` as both `text` and `kana`.
/// Cloneable async closure called both directly and as the
/// `:finder` re-entry through [`find_word_as_hiragana`].
pub type OrAsHiraganaFinder<'a> = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<FindWordRows, sqlx::Error>> + Send + 'a>>
        + Send
        + Sync
        + 'a,
>;

#[derive(Debug, Clone)]
pub enum OrAsHiraganaRows {
    Direct(FindWordRows),
    AsHiragana(Vec<ProxyText>),
}

pub async fn or_as_hiragana<'a>(
    ctx: &'a KaniranContext,
    word: &str,
    fn_: OrAsHiraganaFinder<'a>,
) -> Result<Option<OrAsHiraganaRows>, sqlx::Error> {
    // dict-grammar.lisp:98 (let ((result (apply fn word args))) …)
    let result = fn_(word.to_string()).await?;
    let result_empty = match &result {
        FindWordRows::Kana(rows) => rows.is_empty(),
        FindWordRows::Kanji(rows) => rows.is_empty(),
    };
    if !result_empty {
        return Ok(Some(OrAsHiraganaRows::Direct(result)));
    }
    // dict-grammar.lisp:100 (find-word-as-hiragana word :finder (lambda (w) (apply fn w args)))
    let fn_clone = Arc::clone(&fn_);
    let finder: HiraganaFinder<'a> = Box::new(move |w| fn_clone(w));
    let proxies = find_word_as_hiragana(ctx, word, &[], Some(finder)).await?;
    if proxies.is_empty() {
        Ok(None)
    } else {
        Ok(Some(OrAsHiraganaRows::AsHiragana(proxies)))
    }
}

/// Port of `ichiran/dict:find-word-with-suffix` (`dict-grammar.lisp:102`).
///
/// Collects the [`find_word_full`] outputs whose `seq` is a compound
/// (a list) and whose last element's seq maps to one of
/// `suffix_classes` in [`SUFFIX_CLASS`].
///
/// [`find_word_full`]: crate::dict::path::find_word_full
/// [`SUFFIX_CLASS`]: crate::dict::grammar::suffix::constants::suffix_class
pub async fn find_word_with_suffix(
    ctx: &KaniranContext,
    wordstr: &str,
    suffix_classes: &[&str],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let words = find_word_full(ctx, wordstr, false, None).await?;
    let class_map = suffix_class(ctx);
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    for word in words {
        // dict-grammar.lisp:104 (seq word)
        let word_seq = seq(&word);
        // dict-grammar.lisp:105 (and (listp seq) (gethash (car (last seq)) *suffix-class*))
        let class: Option<&String> = match word_seq {
            Some(WordInfoSeq::Multi(elems)) => match elems.last() {
                Some(Some(WordInfoSeq::Single(i))) => class_map.get(i),
                // nested compound or nil last element — hash lookup misses
                _ => None,
            },
            _ => None,
        };
        // dict-grammar.lisp:106 (when (and suffix-class (find suffix-class suffix-classes)) collect word)
        if let Some(cls) = class {
            if suffix_classes.contains(&cls.as_str()) {
                out.push(word);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
