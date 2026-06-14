use crate::conn::kani_backend::KaniBackend;
use crate::characters::char_class::{count_char_class, CharClass};
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj::{get_conj_data, ConjData, FromOrConjIds};
use crate::dict::counters::methods::text;
use crate::dict::dao::{Entry, KanaText, KanjiText, WordConjugations};
use crate::dict::grammar::synergy::Synergy;
use crate::dict::kani_lite_segment_list::KaniLiteSegmentList;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::path::{PathElement, SegmentList, TopArray, TopArrayItem};
use crate::dict::readings::{best_kanji_conj, get_original_text_star_};
use crate::dict::scoring::score::Segment;
use crate::dict::text_classes::{CompoundText, ProxyText, ScoreMod};
use crate::dict::word_info::WordInfo;
use crate::dict::word_info_str::{reading_str_star_, word_info_reading_str};
use crate::numbers::constants::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
use crate::numbers::kanji_form::number_to_kanji;
use std::borrow::Cow;

/// Port of `ichiran/dict:adjoin-word` (`dict.lisp:632`).
///
/// Builds a compound word from `word1` and `word2`: when `word1` is a
/// simple text it mints a fresh `compound-text`; when it is already a
/// compound it extends that compound (concatenating text/kana and
/// growing `score-mod`). Defaults `text`/`kana` to the concatenation
/// of the two inputs and `score-mod` to `0`.
pub fn adjoin_word(
    ctx: &KaniranContext,
    word1: KaniWordDispatchEnum,
    word2: KaniSimpleTextDispatchEnum,
    text: Option<String>,
    kana: Option<String>,
    score_mod: Option<ScoreMod>,
    score_base: Option<KaniWordDispatchEnum>,
) -> Result<CompoundText, crate::conn::KaniDbError> {
    // dict.lisp:635-640 (defmethod adjoin-word :around (t t))
    let resolved_text = match text {
        Some(t) => t,
        None => {
            let word2_as_word = word2.to_word();
            let t1 = get_text(&word1);
            let t2 = get_text(&word2_as_word);
            format!("{}{}", t1, t2)
        }
    };
    let resolved_kana = match kana {
        Some(k) => k,
        None => {
            let word2_as_word = word2.to_word();
            // dict.lisp:638 — (concatenate 'string (get-kana word1) (get-kana word2)).
            // Upstream `(concatenate 'string nil ...)` accepts nil as
            // the empty sequence; the Rust `Option<String>` from
            // `get_kana` mirrors that with `.unwrap_or_default()`.
            let k1 = get_kana(ctx, &word1)?.unwrap_or_default();
            let k2 = get_kana(ctx, &word2_as_word)?.unwrap_or_default();
            format!("{}{}", k1, k2)
        }
    };
    // dict.lisp:639 — (or score-mod 0).
    // `(or score-mod 0)` evaluates to 0 only when score-mod is nil;
    // any truthy value (integer literal or `(constantly N)` closure)
    // passes through unchanged. The Rust port collapses `None` and
    // the integer-literal default to `ScoreMod::Single(0)`.
    let resolved_score_mod = score_mod.unwrap_or(ScoreMod::Single(0));

    match word1 {
        // dict.lisp:642-645 (defmethod adjoin-word ((word1 simple-text) (word2 simple-text)))
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Proxy(_) => {
            let word2_as_word = word2.to_word();
            // dict.lisp:644 — `:primary word1 :words (list word1 word2)`.
            // Lisp aliases the same word1 cell into both slots; the
            // Rust port clones into `primary` and moves the original
            // into `words`.
            let primary = Box::new(word1.clone());
            Ok(CompoundText {
                text: resolved_text,
                kana: resolved_kana,
                primary,
                words: vec![word1, word2_as_word],
                score_base: score_base.map(Box::new),
                score_mod: resolved_score_mod,
            })
        }
        // dict.lisp:647-652 (defmethod adjoin-word ((word1 compound-text) (word2 simple-text)))
        KaniWordDispatchEnum::Compound(mut compound) => {
            // dict.lisp:649 — setf text/kana on word1.
            compound.text = resolved_text;
            compound.kana = resolved_kana;
            // dict.lisp:650 — (append s-words (list word2)).
            compound.words.push(word2.to_word());
            // dict.lisp:651 — (funcall (if (listp s-score-mod) 'cons 'list)
            //                          score-mod s-score-mod).
            // `cons` prepends onto an existing list; `list` wraps two
            // non-list values into a fresh 2-list. Both branches end
            // with new value at index 0 and old at index 1+.
            compound.score_mod = match compound.score_mod {
                ScoreMod::Stack(stack) => {
                    let mut new_stack = Vec::with_capacity(stack.len() + 1);
                    new_stack.push(resolved_score_mod);
                    new_stack.extend(stack);
                    ScoreMod::Stack(new_stack)
                }
                other @ (ScoreMod::Single(_) | ScoreMod::Constant(_)) => {
                    ScoreMod::Stack(vec![resolved_score_mod, other])
                }
            };
            // dict.lisp:647 — `&allow-other-keys` drops :score-base;
            // word1's score-base slot is unchanged. The `score_base`
            // parameter is discarded in this branch.
            let _ = score_base;
            // dict.lisp:652 — `word1` is returned.
            Ok(compound)
        }
        // dict.lisp:632 — no method specialized on counter-text.
        // Upstream would signal `no-applicable-method`.
        KaniWordDispatchEnum::Counter(_) => {
            unreachable!(
                "adjoin-word has no method specialized on counter-text \
                 (upstream signals no-applicable-method)"
            )
        }
    }
}

/// Port of `ichiran/dict:apply-score-mod` (`dict.lisp:735-742`).
///
/// Computes a compound word's score modifier: an integer modifier
/// gives `score * score-mod * len`, a constant modifier returns its
/// captured value, and a list modifier sums the result over each
/// element.
pub fn apply_score_mod(score_mod: &ScoreMod, score: i64, len: i64) -> i64 {
    match score_mod {
        // dict.lisp:737-738 — (:method ((score-mod integer) score len)
        //                       (* score score-mod len))
        ScoreMod::Single(n) => score * n * len,
        // dict.lisp:739-740 — (:method ((score-mod function) score len)
        //                       (funcall score-mod score)).
        // Every reachable upstream value is (constantly N), which
        // ignores its argument and returns N.
        ScoreMod::Constant(n) => *n,
        // dict.lisp:741-742 — (:method ((score-mod list) score len)
        //                       (reduce '+ score-mod
        //                         :key (lambda (sm) (apply-score-mod sm score len))))
        ScoreMod::Stack(stack) => stack.iter().map(|sm| apply_score_mod(sm, score, len)).sum(),
    }
}

/// Port of `ichiran/dict:get-array` (`dict.lisp:1160`).
///
/// Returns the populated prefix of a [`TopArray`]'s backing storage:
/// the whole array when `count` reaches its length, else the first
/// `count` slots.
pub fn get_array(obj: &TopArray) -> &[Option<TopArrayItem>] {
    if obj.count >= obj.array.len() {
        &obj.array
    } else {
        &obj.array[0..obj.count]
    }
}

/// Port of `ichiran/dict:get-kana` (gf — `dict.lisp:12-13`).
///
/// Generic function returning the most popular kana representation
/// for a word, dispatching over the word variant.
pub fn get_kana(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, crate::conn::KaniDbError> {
    match obj {
        // simple-text family handles its own `:around` internally
        // (dict.lisp:80-84) — see [`KaniSimpleTextDispatchEnum::get_kana`].
        // The clone wraps a borrowed simple-text variant into the
        // family enum; the family method then implements both the
        // `:around` and the primary `call-next-method`.
        KaniWordDispatchEnum::Kanji(k) => {
            KaniSimpleTextDispatchEnum::Kanji(k.clone())
                .get_kana(ctx)
                
        }
        KaniWordDispatchEnum::Kana(k) => {
            KaniSimpleTextDispatchEnum::Kana(k.clone())
                .get_kana(ctx)
                
        }
        KaniWordDispatchEnum::Proxy(p) => {
            KaniSimpleTextDispatchEnum::Proxy(p.clone())
                .get_kana(ctx)
                
        }
        // counter-text family handles its own `:around` (suffix
        // append) and per-subclass overrides internally — see
        // [`Counter::get_kana`].
        KaniWordDispatchEnum::Counter(c) => Ok(Some(c.get_kana())),
        // dict.lisp:610 (kana :reader get-kana :initarg :kana) on compound-text
        KaniWordDispatchEnum::Compound(c) => Ok(Some(c.kana.clone())),
    }
}

/// Port of `ichiran/dict:get-kanji` (gf — `dict.lisp:15-16`).
///
/// Generic function returning the most popular kanji representation
/// for a word, dispatching over the word variant.
pub fn get_kanji(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, crate::conn::KaniDbError> {
    match obj {
        // dict.lisp:108-109 (defmethod get-kanji ((obj kanji-text))) — (text obj)
        KaniWordDispatchEnum::Kanji(k) => Ok(Some(k.text.clone())),
        // dict.lisp:153-155 (defmethod get-kanji ((obj kana-text)))
        // (let ((bk (best-kanji-conj obj))) (unless (eql bk :null) bk))
        KaniWordDispatchEnum::Kana(k) => best_kanji_conj(ctx, k),
        // dict-counters.lisp:61-62 (defmethod get-kanji ((obj counter-text)))
        // (concatenate 'string (number-to-kanji (number-value obj)) (counter-text obj))
        KaniWordDispatchEnum::Counter(c) => {
            let base = c.base();
            let prefix = number_to_kanji(base.number, DIGIT_KANJI_DEFAULT, POWER_KANJI, false);
            Ok(Some(format!("{}{}", prefix, base.text)))
        }
        KaniWordDispatchEnum::Proxy(_) | KaniWordDispatchEnum::Compound(_) => {
            unreachable!("get-kanji has no method on proxy-text / compound-text (dict.lisp:15)")
        }
    }
}

impl Entry {
    /// `get-kanji` method body — `dict.lisp:51-53`:
    ///
    /// ```lisp
    /// (defmethod get-kanji ((obj entry))
    ///   (when (> (n-kanji obj) 0)
    ///     (text (car (select-dao 'kanji-text (:and (:= 'seq (seq obj)) (:= 'ord 0)))))))
    /// ```
    ///
    /// Returns the `text` of the entry's headword kanji row at
    /// `ord = 0` when the entry has any kanji writings; `None`
    /// otherwise.
    ///
    /// Diverges from the upstream lambda list `(obj)` only by taking
    /// `&KaniranContext` for the database handle, replacing the
    /// upstream dynamic `*connection*` per
    /// [`crate::conn::kani_context`]. `None` mirrors upstream falling
    /// off the `when` when `n-kanji = 0`; a missing `ord = 0` row
    /// propagates as [`crate::conn::KaniDbError::RowNotFound`], matching upstream
    /// erroring on `(text nil)`.
    pub fn get_kanji(&self, ctx: &KaniranContext) -> Result<Option<String>, crate::conn::KaniDbError> {
        if self.n_kanji <= 0 {
            return Ok(None);
        }
        // None → RowNotFound mirrors the original fetch_one (see the
        // doc comment above on missing ord = 0 rows).
        let text: String = ctx
            .store
            .headword_kanji_text(self.seq)
            ?
            .ok_or(crate::conn::KaniDbError::RowNotFound)?;
        Ok(Some(text))
    }
}

/// Port of `ichiran/dict:get-original-text` (gf — `dict.lisp:394`).
///
/// Returns the unconjugated reading rows for a `reading`: for
/// simple-text, resolves `(text, seq)` pairs through
/// [`get_original_text_star_`] and looks each up in `kanji_text` or
/// `kana_text` per word type; for proxy-text, recurses on its source.
pub fn get_original_text(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_data: Option<&[ConjData]>,
) -> Result<Vec<KaniSimpleTextDispatchEnum>, crate::conn::KaniDbError> {
    match reading {
        // dict.lisp:589-590 (defmethod get-original-text ((reading proxy-text)))
        KaniSimpleTextDispatchEnum::Proxy(p) => {
            get_original_text(ctx, &p.source, conj_data)
        }
        // dict.lisp:396-400 (defmethod get-original-text ((reading simple-text)))
        KaniSimpleTextDispatchEnum::Kanji(_) | KaniSimpleTextDispatchEnum::Kana(_) => {
            get_original_text_simple_text_arm(ctx, reading, conj_data)
        }
    }
}

fn get_original_text_simple_text_arm(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_data: Option<&[ConjData]>,
) -> Result<Vec<KaniSimpleTextDispatchEnum>, crate::conn::KaniDbError> {
    let (seq_value, conjugations, reading_text, word_type) = match reading {
        KaniSimpleTextDispatchEnum::Kanji(k) => (
            k.seq,
            &k.state.conjugations,
            k.text.as_str(),
            WordType::Kanji,
        ),
        KaniSimpleTextDispatchEnum::Kana(k) => (
            k.seq,
            &k.state.conjugations,
            k.text.as_str(),
            WordType::Kana,
        ),
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
            owned_cd = get_conj_data(ctx, seq_value, from_or_conj_ids, &[reading_text])?;
            &owned_cd
        }
    };

    let orig_texts = get_original_text_star_(ctx, cd, &[reading_text])?;

    let mut rows: Vec<KaniSimpleTextDispatchEnum> = Vec::new();
    for (txt, seq_n) in orig_texts {
        // dict.lisp:399-400 ((select-dao table (:and (:= 'seq seq) (:= 'text txt))))
        match word_type {
            WordType::Kanji => {
                let fetched: Vec<KanjiText> =
                    ctx.store.kanji_texts_by_seq_and_text(seq_n, &txt)?;
                for row in fetched {
                    rows.push(KaniSimpleTextDispatchEnum::Kanji(row));
                }
            }
            WordType::Kana => {
                let fetched: Vec<KanaText> =
                    ctx.store.kana_texts_by_seq_and_text(seq_n, &txt)?;
                for row in fetched {
                    rows.push(KaniSimpleTextDispatchEnum::Kana(row));
                }
            }
            WordType::Gap => {
                unreachable!("simple-text variants always have word-type :kanji or :kana")
            }
        }
    }
    Ok(rows)
}

/// Port of `ichiran/dict:get-segment-score` (`dict.lisp:1040`).
///
/// Generic-function dispatcher returning the segment-style score of a
/// [`Segment`], [`SegmentList`], or [`Synergy`]: the segment's own
/// score, the first segment's score (or 0) for a list, or the synergy's
/// score. `None` when the score slot is unset (before `gen-score`).
#[derive(Debug, Clone)]
pub enum KaniSegmentScoreArg<'a> {
    Segment(&'a Segment),
    SegmentList(&'a SegmentList),
    KaniLiteSegmentList(&'a KaniLiteSegmentList),
    Synergy(&'a Synergy),
}

pub fn get_segment_score(seg: &KaniSegmentScoreArg) -> Option<i32> {
    match seg {
        // dict.lisp:1042-1043 (:method ((seg segment)))
        KaniSegmentScoreArg::Segment(s) => s.score,
        // dict.lisp:1044-1046 (:method ((seg-list segment-list)))
        KaniSegmentScoreArg::SegmentList(sl) => match sl.segments.first() {
            Some(first) => first.score,
            None => Some(0),
        },
        // Same shape as SegmentList arm, but reads the precomputed
        // `score` off the lite segment instead of dereffing into
        // `info` / `Segment.score`.
        KaniSegmentScoreArg::KaniLiteSegmentList(sl) => match sl.segments.first() {
            Some(first) => first.score,
            None => Some(0),
        },
        // dict-grammar.lisp:715-716 (defmethod get-segment-score ((syn synergy)))
        KaniSegmentScoreArg::Synergy(syn) => Some(syn.score),
    }
}

/// Port of `ichiran/dict:get-text` (gf — `dict.lisp:18-20`).
///
/// Returns the most popular text representation (kanji or kana) for a
/// word; the default method delegates to [`crate::dict::counters::methods::text`].
pub fn get_text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    text(obj)
}

/// Port of `ichiran/dict:reading-str` (gf — `dict.lisp:1588-1592`, `1739-1743`).
///
/// Formats the human-readable `"kanji 【kana】"` string for a reading,
/// dispatching on a `simple-text`, an integer seq, or a `word-info`.
impl WordInfo {
    // dict.lisp:1739-1743 (defmethod reading-str ((word-info word-info)) / ((word-info list)))
    pub fn reading_str(&self) -> Option<String> {
        word_info_reading_str(self)
    }
}

impl KaniSimpleTextDispatchEnum {
    // dict.lisp:1589-1590 (defmethod reading-str ((obj simple-text)))
    pub fn reading_str(&self, ctx: &KaniranContext) -> Result<Option<String>, crate::conn::KaniDbError> {
        let kanji = get_kanji(ctx, &self.to_word())?;
        let kana = self.get_kana(ctx)?;
        Ok(reading_str_star_(kanji.as_deref(), kana.as_deref()))
    }
}

/// Port of `ichiran/dict:register-item` (`dict.lisp:1148-1158`).
///
/// Inserts `(score, payload)` into [`TopArray`]'s backing array,
/// maintaining the highest-score-first order by shifting lower-scored
/// entries down. `count` increments every call even when the array is
/// at capacity (the lowest-scored entry is dropped). Equal scores: the
/// new item lands BELOW the existing one.
pub fn register_item(obj: &mut TopArray, score: i32, payload: Vec<PathElement>) {
    // dict.lisp:1151 (let ((item (make-top-array-item :score score :payload payload)) (len ...)))
    let mut item: Option<TopArrayItem> = Some(TopArrayItem { score, payload });
    let len = obj.array.len();
    // dict.lisp:1153 (loop for idx from (min count len) downto 0 ...)
    let start = obj.count.min(len);
    let mut idx = start;
    loop {
        // dict.lisp:1154 (for prev-item = (when (> idx 0) (aref array (1- idx))))
        // The slot value is itself an Option (initialize-instance fills with nil);
        // both "idx == 0" and "slot is nil" collapse to None here.
        let prev_score = if idx > 0 {
            obj.array[idx - 1].as_ref().map(|prev| prev.score)
        } else {
            None
        };
        // dict.lisp:1155 (for done = (or (not prev-item) (>= (tai-score prev-item) score)))
        let done = match prev_score {
            None => true,
            Some(prev) => prev >= score,
        };
        // dict.lisp:1156 (when (< idx len) do (setf (aref array idx) (if done item prev-item)))
        if idx < len {
            obj.array[idx] = if done {
                item.take()
            } else {
                obj.array[idx - 1].take()
            };
        }
        // dict.lisp:1157 (until done)
        if done {
            break;
        }
        idx -= 1;
    }
    // dict.lisp:1158 (incf count)
    obj.count += 1;
}

/// Port of `ichiran/dict:score-base` (`dict.lisp:669-672`).
///
/// Returns the word that drives `compound-text` scoring — the
/// `score-base` slot when set, otherwise the head reading in `primary`.
pub fn score_base(word: &CompoundText) -> &KaniWordDispatchEnum {
    word.score_base.as_deref().unwrap_or(&*word.primary)
}

/// Port of `ichiran/dict:true-kana` (`dict.lisp:560-562`).
///
/// Returns the kana writing for a reading via [`get_kana`], descending
/// through any `proxy-text` wrappers to the underlying source first.
pub fn true_kana(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, crate::conn::KaniDbError> {
    match obj {
        // dict.lisp:562 (:method ((obj proxy-text)) (true-kana (source obj)))
        KaniWordDispatchEnum::Proxy(p) => {
            let leaf = true_kana_unwrap_proxy_chain(p);
            let lifted = match leaf {
                KaniSimpleTextDispatchEnum::Kanji(k) => KaniWordDispatchEnum::Kanji(k.clone()),
                KaniSimpleTextDispatchEnum::Kana(k) => KaniWordDispatchEnum::Kana(k.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => {
                    unreachable!("unwrap_proxy_chain terminates at Kanji or Kana")
                }
            };
            get_kana(ctx, &lifted)
        }
        // dict.lisp:561 (:method (obj) (get-kana obj))
        other => get_kana(ctx, other),
    }
}

fn true_kana_unwrap_proxy_chain(start: &ProxyText) -> &KaniSimpleTextDispatchEnum {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
            _ => return current,
        }
    }
}

/// Port of `ichiran/dict:true-kanji` (`dict.lisp:564-566`).
///
/// Returns the kanji writing for a reading via [`get_kanji`],
/// descending through any `proxy-text` wrappers to the underlying
/// source first.
pub fn true_kanji(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, crate::conn::KaniDbError> {
    match obj {
        // dict.lisp:566 (:method ((obj proxy-text)) (true-kanji (source obj)))
        KaniWordDispatchEnum::Proxy(p) => {
            let lifted = match true_kanji_unwrap_proxy_chain(p) {
                KaniSimpleTextDispatchEnum::Kanji(k) => KaniWordDispatchEnum::Kanji(k.clone()),
                KaniSimpleTextDispatchEnum::Kana(k) => KaniWordDispatchEnum::Kana(k.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => {
                    unreachable!("unwrap_proxy_chain terminates at Kanji or Kana")
                }
            };
            get_kanji(ctx, &lifted)
        }
        // dict.lisp:565 (:method (obj) (get-kanji obj))
        other => get_kanji(ctx, other),
    }
}

fn true_kanji_unwrap_proxy_chain(start: &ProxyText) -> &KaniSimpleTextDispatchEnum {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
            _ => return current,
        }
    }
}

/// Port of `ichiran/dict:true-text` (`dict.lisp:555-558`).
///
/// Returns the surface text of a reading via [`crate::dict::counters::methods::text`],
/// descending through any `proxy-text` wrappers to the underlying
/// source first.
pub fn true_text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    match obj {
        KaniWordDispatchEnum::Proxy(p) => Cow::Borrowed(true_text_unwrap_proxy_chain(p)),
        other => text(other),
    }
}

fn true_text_unwrap_proxy_chain(start: &ProxyText) -> &str {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Kanji(k) => return &k.text,
            KaniSimpleTextDispatchEnum::Kana(k) => return &k.text,
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
        }
    }
}

/// Port of `ichiran/dict:word-conj-data` (`dict.lisp:654`).
///
/// Returns the conjugation data for a word via
/// [`crate::dict::conj::get_conj_data`] — recursing into the last
/// word for compounds and yielding nothing for counters.
pub fn word_conj_data(
    ctx: &KaniranContext,
    word: &KaniWordDispatchEnum,
) -> Result<Vec<ConjData>, crate::conn::KaniDbError> {
    match word {
        // dict-counters.lisp:87 (defmethod word-conj-data ((obj counter-text)) nil)
        KaniWordDispatchEnum::Counter(_) => Ok(Vec::new()),

        // dict.lisp:660-661 (defmethod word-conj-data ((word compound-text)))
        KaniWordDispatchEnum::Compound(c) => {
            let last = c
                .words
                .last()
                .expect("compound-text always has at least one word (adjoin-word ctor)");
            word_conj_data(ctx, last)
        }

        // dict.lisp:657-658 (defmethod word-conj-data ((word simple-text)))
        KaniWordDispatchEnum::Kanji(k) => {
            word_conj_data_simple_text_arm(ctx, k.seq, &k.state.conjugations, &k.text)
        }
        KaniWordDispatchEnum::Kana(k) => {
            word_conj_data_simple_text_arm(ctx, k.seq, &k.state.conjugations, &k.text)
        }
        KaniWordDispatchEnum::Proxy(p) => {
            // dict.lisp:657-658 — proxy's seq / word-conjugations / true-text gfs
            // each delegate to (source obj); the leaf is a kanji-text or kana-text
            // and supplies all three slot reads.
            let leaf = leaf_of(p);
            let (seq, conj, text) = leaf_slots(leaf);
            word_conj_data_simple_text_arm(ctx, seq, conj, text)
        }
    }
}

fn word_conj_data_simple_text_arm(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: &Option<WordConjugations>,
    true_text: &str,
) -> Result<Vec<ConjData>, crate::conn::KaniDbError> {
    let from_or_conj_ids = match conjugations {
        None => FromOrConjIds::All,
        Some(WordConjugations::Root) => FromOrConjIds::Root,
        Some(WordConjugations::Ids(ids)) => FromOrConjIds::ConjIds(ids.clone()),
    };
    get_conj_data(ctx, seq, from_or_conj_ids, &[true_text])
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

fn leaf_slots(leaf: &KaniSimpleTextDispatchEnum) -> (i32, &Option<WordConjugations>, &str) {
    match leaf {
        KaniSimpleTextDispatchEnum::Kanji(k) => (k.seq, &k.state.conjugations, &k.text),
        KaniSimpleTextDispatchEnum::Kana(k) => (k.seq, &k.state.conjugations, &k.text),
        KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!("leaf_of strips proxies"),
    }
}

/// Port of `ichiran/dict:word-type` (gf — `dict.lisp:22-24`).
///
/// Classifies a word's primary script as `:kanji`, `:kana`, or `:gap`
/// — counters resolve by whether their rendered text holds any kanji
/// char; proxies and compounds recurse on their source / primary word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    Kanji,
    Kana,
    Gap,
}

pub fn word_type(obj: &KaniWordDispatchEnum) -> WordType {
    match obj {
        KaniWordDispatchEnum::Kanji(_) => WordType::Kanji,
        KaniWordDispatchEnum::Kana(_) => WordType::Kana,
        KaniWordDispatchEnum::Counter(_) => {
            let t = text(obj);
            if count_char_class(&t, CharClass::KanjiChar) > 0 {
                WordType::Kanji
            } else {
                WordType::Kana
            }
        }
        KaniWordDispatchEnum::Proxy(p) => word_type_simple(&p.source),
        KaniWordDispatchEnum::Compound(c) => word_type(&c.primary),
    }
}

fn word_type_simple(obj: &KaniSimpleTextDispatchEnum) -> WordType {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(_) => WordType::Kanji,
        KaniSimpleTextDispatchEnum::Kana(_) => WordType::Kana,
        KaniSimpleTextDispatchEnum::Proxy(p) => word_type_simple(&p.source),
    }
}

/// Port of `(setf word-conjugations)` — setter half of the
/// `:accessor word-conjugations` slot option on `simple-text`
/// (`dict.lisp:70`).
///
/// Stores the conjugations on a word, recursing to a proxy's source
/// or a compound's last word.
pub fn set_word_conjugations(word: &mut KaniWordDispatchEnum, value: Option<WordConjugations>) {
    match word {
        KaniWordDispatchEnum::Kanji(k) => k.state.conjugations = value,
        KaniWordDispatchEnum::Kana(k) => k.state.conjugations = value,
        KaniWordDispatchEnum::Proxy(p) => set_simple_text(&mut p.source, value),
        KaniWordDispatchEnum::Compound(c) => match c.words.last_mut() {
            Some(last) => set_word_conjugations(last, value),
            // (car (last nil)) → nil; (setf (word-conjugations nil) v)
            // → no-applicable-method on nil upstream.
            None => panic!(
                "(setf word-conjugations) on compound-text with empty words: no applicable method"
            ),
        },
        KaniWordDispatchEnum::Counter(_) => {
            panic!("(setf word-conjugations) on counter-text: no applicable method")
        }
    }
}

fn set_simple_text(simple: &mut KaniSimpleTextDispatchEnum, value: Option<WordConjugations>) {
    match simple {
        KaniSimpleTextDispatchEnum::Kanji(k) => k.state.conjugations = value,
        KaniSimpleTextDispatchEnum::Kana(k) => k.state.conjugations = value,
        KaniSimpleTextDispatchEnum::Proxy(p) => set_simple_text(&mut p.source, value),
    }
}

/// Port of the `word-conjugations` accessor on `simple-text`
/// (`dict.lisp:70`), with proxy/counter/compound overrides.
///
/// Returns a reading's `word-conjugations` slot — recursing through
/// proxy chains and into a compound's last word, nil for counters.
pub fn word_conjugations(word: &KaniWordDispatchEnum) -> Option<WordConjugations> {
    match word {
        // dict.lisp:70 — `:accessor word-conjugations` slot on simple-text.
        KaniWordDispatchEnum::Kanji(k) => k.state.conjugations.clone(),
        KaniWordDispatchEnum::Kana(k) => k.state.conjugations.clone(),
        // dict.lisp:568-569 (defmethod word-conjugations ((obj proxy-text))
        //   (word-conjugations (source obj))) — descend through proxy chain.
        KaniWordDispatchEnum::Proxy(p) => proxy_chain_conjugations(p),
        // dict-counters.lisp:85 (defmethod word-conjugations ((obj counter-text)) nil)
        KaniWordDispatchEnum::Counter(_) => None,
        // dict.lisp:663-664 (defmethod word-conjugations ((word compound-text))
        //   (word-conjugations (car (last (words word)))))
        KaniWordDispatchEnum::Compound(c) => match c.words.last() {
            Some(last) => word_conjugations(last),
            None => None,
        },
    }
}

fn proxy_chain_conjugations(p: &ProxyText) -> Option<WordConjugations> {
    let mut current: &KaniSimpleTextDispatchEnum = &p.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Kanji(k) => return k.state.conjugations.clone(),
            KaniSimpleTextDispatchEnum::Kana(k) => return k.state.conjugations.clone(),
            KaniSimpleTextDispatchEnum::Proxy(inner) => current = &inner.source,
        }
    }
}

#[cfg(test)]
mod tests;
