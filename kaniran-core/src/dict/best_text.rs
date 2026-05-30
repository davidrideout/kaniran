//! Port of the dict.lisp best-text / get-original-text layer — the
//! polymorphic get-{text,kana,kanji}, true-{text,kana,kanji},
//! get-original-text[-once,*], query-parents-{kanji,kana},
//! best-{kana,kanji}-conj, get-kanji-words, get-kanji-kana-old,
//! get-array, map-word-info-kana, and word-info-reading[-str] /
//! word-info-str helpers.

pub use get_original_text_inner::*;
pub use get_original_text_once_inner::*;
pub use get_original_text_star__inner::*;
pub use query_parents_kanji_inner::*;
pub use query_parents_kana_inner::*;
pub use best_kana_conj_inner::*;
pub use best_kanji_conj_inner::*;
pub use get_text_inner::*;
pub use get_kana_inner::*;
pub use get_kanji_inner::*;
pub use true_text_inner::*;
pub use true_kana_inner::*;
pub use true_kanji_inner::*;
pub use get_kanji_words_inner::*;
pub use get_kanji_kana_old_inner::*;
pub use get_array_inner::*;
pub use map_word_info_kana_inner::*;
pub use word_info_reading_inner::*;
pub use word_info_reading_str_inner::*;
pub use word_info_str_inner::*;

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_original_text_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data::ConjData;
use crate::dict::conj_data::{get_conj_data, FromOrConjIds};
use crate::dict::best_text::get_original_text_star_;
use crate::dict::dao::KanaText;
use crate::dict::kani::KaniSimpleTextDispatchEnum;
use crate::dict::dao::KanjiText;
use crate::dict::text_classes::WordConjugations;
use crate::dict::word_info::WordType;

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
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_original_text_once_inner {
use crate::dict::conj_data::ConjData;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::conj_data::make_conj_data;

    fn cd(pairs: &[(&str, &str)]) -> ConjData {
        make_conj_data(
            None,
            None,
            None,
            None,
            pairs
                .iter()
                .map(|(txt, src_txt)| (txt.to_string(), src_txt.to_string()))
                .collect(),
        )
    }

    /// REPL fixtures (.103, `ichiran/dict::get-original-text-once` over
    /// `make-conj-data` built from the real 食べる conj-source-reading
    /// rows), 2026-05-24. Output order tracks `src-map` iteration order,
    /// not `texts` order — both two-text rows below collect
    /// `("たべる" "食べる")` regardless of how the texts are ordered.
    #[test]
    fn get_original_text_once_fixtures() {
        let cd1 = cd(&[
            ("たべます", "たべる"),
            ("喰べます", "喰べる"),
            ("食べます", "食べる"),
        ]);
        let cd2 = cd(&[
            ("たべない", "たべる"),
            ("喰べない", "喰べる"),
            ("食べない", "食べる"),
        ]);
        let cases: &[(&[ConjData], &[&str], &[&str])] = &[
            (std::slice::from_ref(&cd1), &["食べます"], &["食べる"]),
            (std::slice::from_ref(&cd1), &["たべます"], &["たべる"]),
            (
                std::slice::from_ref(&cd1),
                &["食べます", "たべます"],
                &["たべる", "食べる"],
            ),
            (
                std::slice::from_ref(&cd1),
                &["たべます", "食べます"],
                &["たべる", "食べる"],
            ),
            (std::slice::from_ref(&cd1), &["xyz"], &[]),
            (std::slice::from_ref(&cd1), &[], &[]),
            (
                std::slice::from_ref(&cd1),
                &["たべます", "喰べます", "食べます"],
                &["たべる", "喰べる", "食べる"],
            ),
            (
                &[cd1.clone(), cd2.clone()],
                &["食べます", "食べない"],
                &["食べる", "食べる"],
            ),
            (std::slice::from_ref(&cd2), &["食べない"], &["食べる"]),
            (&[], &["食べます"], &[]),
        ];
        for (conj_datas, texts, expected) in cases {
            let actual = get_original_text_once(conj_datas, texts);
            let actual_refs: Vec<&str> = actual.iter().map(String::as_str).collect();
            assert_eq!(actual_refs.as_slice(), *expected, "texts={texts:?}");
        }
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_original_text_star__inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data::ConjData;
use crate::dict::conj_data::{get_conj_data, FromOrConjIds};

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
                let inner =
                    Box::pin(get_original_text_star_(ctx, &new_cd, &src_text)).await?;
                out.extend(inner);
            }
        }
    }
    Ok(out)
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod query_parents_kanji_inner {
use crate::conn::kani_context::KaniranContext;

pub async fn query_parents_kanji(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT kt.id, conj.id \
         FROM kanji_text kt, conj_source_reading csr, conjugation conj \
         WHERE conj.seq = $1 \
           AND conj.id = csr.conj_id \
           AND csr.text = $2 \
           AND kt.seq = CASE WHEN conj.via IS NOT NULL THEN conj.via ELSE conj.from END \
           AND kt.text = csr.source_text",
    )
    .bind(seq)
    .bind(text)
    .fetch_all(&ctx.pool)
    .await?;
    Ok(rows)
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod query_parents_kana_inner {
use crate::conn::kani_context::KaniranContext;

pub async fn query_parents_kana(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT kt.id, conj.id \
         FROM kana_text kt, conj_source_reading csr, conjugation conj \
         WHERE conj.seq = $1 \
           AND conj.id = csr.conj_id \
           AND csr.text = $2 \
           AND kt.seq = CASE WHEN conj.via IS NOT NULL THEN conj.via ELSE conj.from END \
           AND kt.text = csr.source_text",
    )
    .bind(seq)
    .bind(text)
    .fetch_all(&ctx.pool)
    .await?;
    Ok(rows)
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod best_kana_conj_inner {
use crate::characters::kanji::kanji_cross_match;
use crate::characters::kanji::kanji_regex;
use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::KanjiText;
use crate::dict::best_text::query_parents_kanji;
use crate::dict::text_classes::WordConjugations;

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
        // fetch_one mirrors upstream: a missing pid would surface as nil
        // from get-dao and crash on the next slot access; propagating
        // the sqlx error preserves that fail-loud stance.
        let parent_kt: KanjiText =
            sqlx::query_as("SELECT * FROM kanji_text WHERE id = $1")
                .bind(pid)
                .fetch_one(&ctx.pool)
                .await?;
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
        let readings: Vec<String> = sqlx::query_scalar(
            "SELECT text FROM conj_source_reading \
             WHERE conj_id = $1 AND source_text = $2",
        )
        .bind(cid)
        .bind(&parent_bk)
        .fetch_all(&ctx.pool)
        .await?;
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
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod best_kanji_conj_inner {
use crate::characters::kanji::kanji_match;
use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::Entry;
use crate::dict::dao::KanaText;
use crate::dict::best_text::query_parents_kana;
use crate::dict::text_classes::WordConjugations;

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
    let entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(obj.seq)
        .fetch_one(&ctx.pool)
        .await?;
    if entry.n_kanji == 0 {
        return Ok(None);
    }

    let parents = query_parents_kana(ctx, obj.seq, &obj.text).await?;
    for (pid, cid) in parents {
        // dict.lisp:465 (for parent-bk = (best-kanji-conj (get-dao 'kana-text pid)))
        let parent_kt: KanaText =
            sqlx::query_as("SELECT * FROM kana_text WHERE id = $1")
                .bind(pid)
                .fetch_one(&ctx.pool)
                .await?;
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
        let readings: Vec<String> = sqlx::query_scalar(
            "SELECT text FROM conj_source_reading \
             WHERE conj_id = $1 AND source_text = $2",
        )
        .bind(cid)
        .bind(&parent_bk)
        .fetch_all(&ctx.pool)
        .await?;
        // dict.lisp:471-473 (some (lambda (reading) (and (kanji-match reading (text obj)) reading)) readings)
        if let Some(hit) = readings.into_iter().find(|r| kanji_match(r, &obj.text)) {
            return Ok(Some(hit));
        }
    }
    Ok(None)
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_text_inner {
use std::borrow::Cow;

use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::counters::dispatchers::text;

pub fn get_text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    text(obj)
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kana_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

pub async fn get_kana(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // simple-text family handles its own `:around` internally
        // (dict.lisp:80-84) — see [`KaniSimpleTextDispatchEnum::get_kana`].
        // The clone wraps a borrowed simple-text variant into the
        // family enum; the family method then implements both the
        // `:around` and the primary `call-next-method`.
        KaniWordDispatchEnum::Kanji(k) => {
            KaniSimpleTextDispatchEnum::Kanji(k.clone())
                .get_kana(ctx).await
        }
        KaniWordDispatchEnum::Kana(k) => {
            KaniSimpleTextDispatchEnum::Kana(k.clone())
                .get_kana(ctx).await
        }
        KaniWordDispatchEnum::Proxy(p) => {
            KaniSimpleTextDispatchEnum::Proxy(p.clone())
                .get_kana(ctx).await
        }
        // counter-text family handles its own `:around` (suffix
        // append) and per-subclass overrides internally — see
        // [`Counter::get_kana`].
        KaniWordDispatchEnum::Counter(c) => Ok(Some(c.get_kana())),
        // dict.lisp:610 (kana :reader get-kana :initarg :kana) on compound-text
        KaniWordDispatchEnum::Compound(c) => Ok(Some(c.kana.clone())),
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kanji_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_text::best_kanji_conj;
use crate::dict::dao::Entry;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::numbers::num_class::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
use crate::numbers::kanji_form::number_to_kanji;

pub async fn get_kanji(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // dict.lisp:108-109 (defmethod get-kanji ((obj kanji-text))) — (text obj)
        KaniWordDispatchEnum::Kanji(k) => Ok(Some(k.text.clone())),
        // dict.lisp:153-155 (defmethod get-kanji ((obj kana-text)))
        // (let ((bk (best-kanji-conj obj))) (unless (eql bk :null) bk))
        KaniWordDispatchEnum::Kana(k) => best_kanji_conj(ctx, k).await,
        // dict-counters.lisp:61-62 (defmethod get-kanji ((obj counter-text)))
        // (concatenate 'string (number-to-kanji (number-value obj)) (counter-text obj))
        KaniWordDispatchEnum::Counter(c) => {
            let base = c.base();
            let prefix = number_to_kanji(base.number, DIGIT_KANJI_DEFAULT, POWER_KANJI, false);
            Ok(Some(format!("{}{}", prefix, base.text)))
        }
        KaniWordDispatchEnum::Proxy(_) | KaniWordDispatchEnum::Compound(_) => {
            unreachable!(
                "get-kanji has no method on proxy-text / compound-text (dict.lisp:15)"
            )
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
    /// propagates as [`sqlx::Error::RowNotFound`], matching upstream
    /// erroring on `(text nil)`.
    pub async fn get_kanji(
        &self,
        ctx: &KaniranContext,
    ) -> Result<Option<String>, sqlx::Error> {
        if self.n_kanji <= 0 {
            return Ok(None);
        }
        let (text,): (String,) = sqlx::query_as(
            "SELECT text FROM kanji_text WHERE seq = $1 AND ord = 0",
        )
        .bind(self.seq)
        .fetch_one(&ctx.pool)
        .await?;
        Ok(Some(text))
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod true_text_inner {
use std::borrow::Cow;

use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::text_classes::ProxyText;
use crate::dict::counters::dispatchers::text;

pub fn true_text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    match obj {
        KaniWordDispatchEnum::Proxy(p) => Cow::Borrowed(unwrap_proxy_chain(p)),
        other => text(other),
    }
}

fn unwrap_proxy_chain(start: &ProxyText) -> &str {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Kanji(k) => return &k.text,
            KaniSimpleTextDispatchEnum::Kana(k) => return &k.text,
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
        }
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod true_kana_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_text::get_kana;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::text_classes::ProxyText;

pub async fn true_kana(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // dict.lisp:562 (:method ((obj proxy-text)) (true-kana (source obj)))
        KaniWordDispatchEnum::Proxy(p) => {
            let leaf = unwrap_proxy_chain(p);
            let lifted = match leaf {
                KaniSimpleTextDispatchEnum::Kanji(k) => KaniWordDispatchEnum::Kanji(k.clone()),
                KaniSimpleTextDispatchEnum::Kana(k) => KaniWordDispatchEnum::Kana(k.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!(
                    "unwrap_proxy_chain terminates at Kanji or Kana"
                ),
            };
            Box::pin(get_kana(ctx, &lifted)).await
        }
        // dict.lisp:561 (:method (obj) (get-kana obj))
        other => Box::pin(get_kana(ctx, other)).await,
    }
}

fn unwrap_proxy_chain(start: &ProxyText) -> &KaniSimpleTextDispatchEnum {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
            _ => return current,
        }
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod true_kanji_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_text::get_kanji;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::text_classes::ProxyText;

pub async fn true_kanji(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // dict.lisp:566 (:method ((obj proxy-text)) (true-kanji (source obj)))
        KaniWordDispatchEnum::Proxy(p) => {
            let lifted = match unwrap_proxy_chain(p) {
                KaniSimpleTextDispatchEnum::Kanji(k) => KaniWordDispatchEnum::Kanji(k.clone()),
                KaniSimpleTextDispatchEnum::Kana(k) => KaniWordDispatchEnum::Kana(k.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!(
                    "unwrap_proxy_chain terminates at Kanji or Kana"
                ),
            };
            get_kanji(ctx, &lifted).await
        }
        // dict.lisp:565 (:method (obj) (get-kanji obj))
        other => get_kanji(ctx, other).await,
    }
}

fn unwrap_proxy_chain(start: &ProxyText) -> &KaniSimpleTextDispatchEnum {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
            _ => return current,
        }
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kanji_words_inner {
use crate::conn::kani_context::KaniranContext;

pub async fn get_kanji_words(
    ctx: &KaniranContext,
    char: &str,
) -> Result<Vec<(i32, String, String, i32)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT e.seq, k.text, r.text, k.common \
         FROM entry AS e, kanji_text AS k, kana_text AS r \
         WHERE e.seq = k.seq \
           AND e.seq = r.seq \
           AND r.text = k.best_kana \
           AND k.common IS NOT NULL \
           AND e.root_p \
           AND k.text LIKE '%' || $1 || '%'",
    )
    .bind(char)
    .fetch_all(&ctx.pool)
    .await
}

#[cfg(test)]
mod tests {
    //! Every assertion is REPL-verified against the .103 SBCL via
    //! `(ichiran/dict::get-kanji-words …)` (2026-05-25 probe).
    //! Run with `-- --test-threads=1` per the DB-test convention.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn row(seq: i32, kanji: &str, kana: &str, common: i32) -> (i32, String, String, i32) {
        (seq, kanji.to_string(), kana.to_string(), common)
    }

    /// The query has no ORDER BY, so the result is an unordered set; both
    /// sides are sorted by seq before comparison. `蜂蜜` carries
    /// `common = 0`, exercising the non-null-but-zero branch of the
    /// `(:not-null 'k.common)` filter.
    #[tokio::test]
    async fn get_kanji_words_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, Vec<(i32, String, String, i32)>)] = &[
            (
                "鯨",
                vec![
                    row(1253270, "鯨", "くじら", 13),
                    row(1253290, "鯨肉", "げいにく", 30),
                    row(1514180, "捕鯨", "ほげい", 6),
                ],
            ),
            ("錐", vec![row(1175930, "円錐", "まるぎり", 44)]),
            (
                "蜂",
                vec![
                    row(1517840, "蜂", "はち", 34),
                    row(1517860, "蜂蜜", "はちみつ", 0),
                    row(1729030, "蜂起", "ほうき", 39),
                ],
            ),
        ];
        for (char, expected) in cases {
            let mut got = get_kanji_words(&ctx, char).await.unwrap();
            // Result is an unordered set (no ORDER BY); sort the whole
            // tuple so the comparison is deterministic even if a char
            // ever yields two rows sharing a seq.
            got.sort();
            let mut expected = expected.clone();
            expected.sort();
            assert_eq!(got, expected, "char={char:?}");
        }
    }

    /// `#\火` (char) and `"火"` (string) hit the same query in upstream;
    /// the Rust port collapses both to `&str`, so a single-character
    /// argument returns the full substring match set.
    #[tokio::test]
    async fn single_char_argument() {
        let ctx = ctx().await;
        let words = get_kanji_words(&ctx, "火").await.unwrap();
        assert_eq!(words.len(), 75);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kanji_kana_old_inner {
use crate::characters::kanji::kanji_regex;
use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::KanaText;
use crate::dict::dao::KanjiText;

pub async fn get_kanji_kana_old(
    ctx: &KaniranContext,
    obj: &KanjiText,
) -> Result<Option<String>, sqlx::Error> {
    let regex = kanji_regex(&obj.text);
    let kts = sqlx::query_as::<_, KanaText>(
        "SELECT * FROM kana_text WHERE seq = $1 ORDER BY ord",
    )
    .bind(obj.seq)
    .fetch_all(&ctx.pool)
    .await?;
    for kt in &kts {
        if regex.is_match(&kt.text).unwrap_or(false) {
            return Ok(Some(kt.text.clone()));
        }
    }
    Ok(kts.into_iter().next().map(|kt| kt.text))
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_array_inner {
use crate::dict::segment::TopArray;
use crate::dict::segment::TopArrayItem;

pub fn get_array(obj: &TopArray) -> &[Option<TopArrayItem>] {
    if obj.count >= obj.array.len() {
        &obj.array
    } else {
        &obj.array[0..obj.count]
    }
}

#[cfg(test)]
mod tests {
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::{PathElement, TopArrayItem};
    use super::*;

    fn dummy_payload(score: i32) -> TopArrayItem {
        TopArrayItem {
            score,
            payload: vec![PathElement::SegmentList(SegmentList {
                segments: vec![],
                start: 0,
                end: 0,
                top: None,
                matches: 0,
            })],
        }
    }

    #[test]
    fn empty_top_array_returns_empty_slice() {
        // REPL: empty len=0
        let ta = TopArray::new(3);
        assert_eq!(get_array(&ta).len(), 0);
    }

    #[test]
    fn partial_returns_first_count_slots() {
        // REPL: after 1 register, len=1, first score=50
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(50));
        ta.count = 1;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].as_ref().unwrap().score, 50);
    }

    #[test]
    fn count_equal_to_len_returns_full() {
        // REPL: after 3, len=3 scores=(100 50 10)
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(100));
        ta.array[1] = Some(dummy_payload(50));
        ta.array[2] = Some(dummy_payload(10));
        ta.count = 3;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr.iter()
                .map(|x| x.as_ref().unwrap().score)
                .collect::<Vec<_>>(),
            vec![100, 50, 10]
        );
    }

    #[test]
    fn count_exceeding_len_returns_full() {
        // REPL: after 4 (overflow), len=3 scores=(999 100 50)
        // count exceeds array.len() but we still return the whole array.
        let mut ta = TopArray::new(3);
        ta.array[0] = Some(dummy_payload(999));
        ta.array[1] = Some(dummy_payload(100));
        ta.array[2] = Some(dummy_payload(50));
        ta.count = 4;
        let arr = get_array(&ta);
        assert_eq!(arr.len(), 3);
        assert_eq!(
            arr.iter()
                .map(|x| x.as_ref().unwrap().score)
                .collect::<Vec<_>>(),
            vec![999, 100, 50]
        );
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod map_word_info_kana_inner {
use crate::dict::word_info::simplify_reading_list;
use crate::dict::word_info::{WordInfo, WordInfoKana};
use crate::characters::text_utils::join;

pub fn map_word_info_kana<F>(fn_: F, word_info: &WordInfo, separator: &str) -> String
where
    F: Fn(&Option<WordInfoKana>) -> String,
{
    let wkana = &word_info.kana;
    match wkana {
        // (listp wkana) is nil for a string -> (funcall fn wkana).
        Some(WordInfoKana::Single(_)) => fn_(wkana),
        // (listp wkana) is t for a list -> (mapcar fn wkana).
        Some(WordInfoKana::Multi(elements)) => {
            let mapped: Vec<String> = elements.iter().map(|element| fn_(element)).collect();
            join(separator, &simplify_reading_list(&mapped))
        }
        // (listp nil) is t -> (mapcar fn nil) = nil -> join over empty = "".
        None => join(separator, &simplify_reading_list(&[])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wi(kana: Option<WordInfoKana>) -> WordInfo {
        WordInfo { kana, ..Default::default() }
    }

    fn single(text: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(text.to_string()))
    }

    /// `fn` analogous to the REPL `string-upcase` probe.
    fn upcase(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.to_uppercase(),
            other => format!("{other:?}"),
        }
    }

    /// `fn` analogous to the REPL `identity` probe (kana element as text).
    fn ident(element: &Option<WordInfoKana>) -> String {
        match element {
            Some(WordInfoKana::Single(text)) => text.clone(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn map_word_info_kana_fixtures() {
        // REPL fixtures (.103, ichiran/dict::map-word-info-kana), 2026-05-23.

        // String branch: (funcall fn wkana).
        assert_eq!(map_word_info_kana(upcase, &wi(single("neko")), "/"), "NEKO");

        // List branch, default separator "/".
        let inu = Some(WordInfoKana::Multi(vec![single("neko"), single("inu")]));
        assert_eq!(map_word_info_kana(upcase, &wi(inu.clone()), "/"), "NEKO/INU");

        // List branch, identity fn -> simplify-reading-list merges the
        // shared de-spaced reading with a MIDDLE_DOT boundary.
        let merge = Some(WordInfoKana::Multi(vec![single("a b"), single("ab")]));
        assert_eq!(map_word_info_kana(ident, &wi(merge), "/"), "a\u{00B7}b");

        // nil kana -> list branch -> "".
        assert_eq!(map_word_info_kana(upcase, &wi(None), "/"), "");

        // Separator override.
        assert_eq!(map_word_info_kana(ident, &wi(inu), "+"), "neko+inu");

        // Single-element list.
        let one = Some(WordInfoKana::Multi(vec![single("neko")]));
        assert_eq!(map_word_info_kana(upcase, &wi(one), "/"), "NEKO");
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod word_info_reading_inner {
use crate::dict::dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::dao::KanjiText;
use crate::dict::word_info::{WordInfo, WordInfoType};
use crate::conn::kani_context::KaniranContext;

pub async fn word_info_reading(
    ctx: &KaniranContext,
    word_info: &WordInfo,
) -> Result<Option<KaniWordDispatchEnum>, sqlx::Error> {
    // (true-text (word-info-true-text word-info)) — the `(and table true-text)`
    // guard fails outright when true-text is nil.
    let true_text = match &word_info.true_text {
        Some(true_text) => true_text,
        None => return Ok(None),
    };
    // (case (word-info-type word-info) (:kanji 'kanji-text) (:kana 'kana-text))
    // then (car (select-dao table (:= 'text true-text)))
    match word_info.kind {
        WordInfoType::Kanji => {
            let row: Option<KanjiText> = sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1")
                .bind(true_text)
                .fetch_optional(&ctx.pool)
                .await?;
            Ok(row.map(KaniWordDispatchEnum::Kanji))
        }
        WordInfoType::Kana => {
            let row: Option<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE text = $1")
                .bind(true_text)
                .fetch_optional(&ctx.pool)
                .await?;
            Ok(row.map(KaniWordDispatchEnum::Kana))
        }
        // (case …) has no :gap clause → table nil → guard fails → nil.
        WordInfoType::Gap => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn wi(kind: WordInfoType, true_text: Option<&str>) -> WordInfo {
        WordInfo {
            kind,
            true_text: true_text.map(str::to_owned),
            ..Default::default()
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::word-info-reading`), 2026-05-25.
    /// Each true-text below has exactly one row in its table, so the
    /// `car` of `select-dao` is deterministic. Covers: the `:kanji`
    /// branch (学校, 図書館), the `:kana` branch (ねこ, きそうてんがい),
    /// the `:gap` type (table nil → None), nil true-text (guard fails →
    /// None), and a true-text with no matching row (select empty → None).
    #[tokio::test]
    async fn word_info_reading_fixtures() {
        let ctx = ctx_from_env().await;

        let cases: &[(WordInfo, Option<(i32, i32, bool)>)] = &[
            // (word-info, Some((seq, id, is_kanji)) | None)
            (wi(WordInfoType::Kanji, Some("学校")), Some((1206730, 9064, true))),
            (wi(WordInfoType::Kanji, Some("図書館")), Some((1370420, 29808, true))),
            (wi(WordInfoType::Kana, Some("ねこ")), Some((1467640, 54168, false))),
            (
                wi(WordInfoType::Kana, Some("きそうてんがい")),
                Some((1219430, 28651, false)),
            ),
            (wi(WordInfoType::Gap, Some("学校")), None),
            (wi(WordInfoType::Kanji, None), None),
            (wi(WordInfoType::Kanji, Some("存在しない漢字列 abcxyz")), None),
        ];

        for (word_info, expected) in cases {
            let result = word_info_reading(&ctx, word_info).await.unwrap();
            match (expected, result) {
                (None, None) => {}
                (Some((seq, id, is_kanji)), Some(KaniWordDispatchEnum::Kanji(row))) => {
                    assert!(*is_kanji, "true_text={:?}: expected kana-text", word_info.true_text);
                    assert_eq!(row.seq, *seq, "true_text={:?}", word_info.true_text);
                    assert_eq!(row.id, *id, "true_text={:?}", word_info.true_text);
                    assert_eq!(&row.text, word_info.true_text.as_ref().unwrap());
                }
                (Some((seq, id, is_kanji)), Some(KaniWordDispatchEnum::Kana(row))) => {
                    assert!(!*is_kanji, "true_text={:?}: expected kanji-text", word_info.true_text);
                    assert_eq!(row.seq, *seq, "true_text={:?}", word_info.true_text);
                    assert_eq!(row.id, *id, "true_text={:?}", word_info.true_text);
                    assert_eq!(&row.text, word_info.true_text.as_ref().unwrap());
                }
                (expected, result) => panic!(
                    "true_text={:?}: expected {expected:?}, got variant mismatch ({})",
                    word_info.true_text,
                    result.is_some()
                ),
            }
        }
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod word_info_reading_str_inner {
use crate::dict::senses::reading_str_star_;
use crate::dict::word_info::{WordInfo, WordInfoKana, WordInfoType};
use std::borrow::Cow;

pub fn word_info_reading_str(word_info: &WordInfo) -> Option<String> {
    if word_info.kind == WordInfoType::Kanji
        || (word_info.counter.is_some() && word_info.seq.is_some())
    {
        let kana = word_info.kana.as_ref().map(princ_kana);
        reading_str_star_(Some(&word_info.text), kana.as_deref())
    } else {
        reading_str_star_(None, Some(&word_info.text))
    }
}

// `~a`/princ rendering of a kana value: a string prints verbatim, a list
// prints `(elem ...)` with nil → NIL and nested lists recursing.
fn princ_kana(kana: &WordInfoKana) -> Cow<'_, str> {
    match kana {
        WordInfoKana::Single(reading) => Cow::Borrowed(reading),
        WordInfoKana::Multi(readings) => {
            let elems: Vec<String> = readings
                .iter()
                .map(|reading| match reading {
                    None => "NIL".to_string(),
                    Some(reading) => princ_kana(reading).into_owned(),
                })
                .collect();
            Cow::Owned(format!("({})", elems.join(" ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::word_info::WordInfoSeq;

    fn wi(
        kind: WordInfoType,
        text: &str,
        kana: Option<WordInfoKana>,
        counter: Option<(String, bool)>,
        seq: Option<WordInfoSeq>,
    ) -> WordInfo {
        WordInfo {
            kind,
            text: text.to_string(),
            kana,
            counter,
            seq,
            ..Default::default()
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::word-info-reading-str`), 2026-05-24.
    /// Covers the kanji-type branch (single-string / list / nested-list-with-nil
    /// / nil kana → `~a` princ rendering), the counter+seq branch reaching the
    /// same body, and the else branch (counter without seq, kana type, gap type).
    #[test]
    fn word_info_reading_str_fixtures() {
        use WordInfoKana::{Multi, Single};
        let single = |reading: &str| Single(reading.to_string());
        let cases: Vec<(WordInfo, &str)> = vec![
            (
                wi(WordInfoType::Kanji, "日本", Some(single("にほん")), None, None),
                "日本 【にほん】",
            ),
            (
                wi(
                    WordInfoType::Kanji,
                    "日本",
                    Some(Multi(vec![Some(single("にほん")), Some(single("にっぽん"))])),
                    None,
                    None,
                ),
                "日本 【(にほん にっぽん)】",
            ),
            (
                wi(
                    WordInfoType::Kanji,
                    "X",
                    Some(Multi(vec![
                        Some(single("あ")),
                        None,
                        Some(Multi(vec![Some(single("い")), Some(single("う"))])),
                    ])),
                    None,
                    None,
                ),
                "X 【(あ NIL (い う))】",
            ),
            (
                wi(WordInfoType::Kanji, "日本", None, None, None),
                "日本 【NIL】",
            ),
            (
                wi(
                    WordInfoType::Kana,
                    "三冊",
                    Some(single("さんさつ")),
                    Some(("3".to_string(), false)),
                    Some(WordInfoSeq::Single(12345)),
                ),
                "三冊 【さんさつ】",
            ),
            (
                wi(
                    WordInfoType::Kana,
                    "三冊",
                    Some(single("さんさつ")),
                    Some(("3".to_string(), false)),
                    None,
                ),
                "三冊",
            ),
            (
                wi(WordInfoType::Kana, "ねこ", Some(single("ねこ")), None, None),
                "ねこ",
            ),
            (
                wi(WordInfoType::Gap, "?", Some(single("?")), None, None),
                "?",
            ),
        ];
        for (word_info, expected) in &cases {
            assert_eq!(
                word_info_reading_str(word_info).as_deref(),
                Some(*expected),
                "text={:?}",
                word_info.text
            );
        }
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod word_info_str_inner {
use std::fmt::Write;

use crate::dict::senses::get_senses_str;
use crate::dict::grammar::suffix_init::get_suffix_description;
use crate::dict::conj_data::print_conj_info;
use crate::dict::text_classes::WordConjugations;
use crate::dict::word_info::{WordInfo, WordInfoSeq};
use crate::conn::kani_context::KaniranContext;

pub async fn word_info_str(
    ctx: &KaniranContext,
    word_info: &WordInfo,
) -> Result<String, sqlx::Error> {
    let mut s = String::new();
    if word_info.alternative {
        // dict.lisp:1775-1779 (loop for wi … for i from 1 when (> i 1) do (terpri s) do (format s "<~a>. " i) (inner wi nil nil))
        for (index, wi) in word_info.components.iter().enumerate() {
            let i = index + 1;
            if i > 1 {
                s.push('\n');
            }
            write!(s, "<{}>. ", i).unwrap();
            inner(ctx, wi, false, false, &mut s).await?;
        }
    } else {
        inner(ctx, word_info, false, false, &mut s).await?;
    }
    Ok(s)
}

// dict.lisp:1748 (labels inner (word-info &optional suffix marker))
async fn inner(
    ctx: &KaniranContext,
    word_info: &WordInfo,
    suffix: bool,
    marker: bool,
    s: &mut String,
) -> Result<(), sqlx::Error> {
    if marker {
        s.push_str(" * ");
    }
    // (princ (reading-str word-info) s)
    s.push_str(word_info.reading_str().as_deref().unwrap_or("NIL"));
    if !word_info.components.is_empty() {
        // dict.lisp:1754 (format s " Compound word: ~{~a~^ + ~}" (mapcar #'word-info-text components))
        let texts: Vec<&str> = word_info
            .components
            .iter()
            .map(|comp| comp.text.as_str())
            .collect();
        write!(s, " Compound word: {}", texts.join(" + ")).unwrap();
        // dict.lisp:1755-1757 (dolist (comp components) (terpri s) (inner comp (not (word-info-primary comp)) t))
        for comp in &word_info.components {
            s.push('\n');
            Box::pin(inner(ctx, comp, !comp.primary, true, s)).await?;
        }
    } else if let Some((value, _ordinal)) = &word_info.counter {
        // dict.lisp:1759-1763 (destructuring-bind (value ordinal) (word-info-counter …) (terpri s) (princ value s) …)
        s.push('\n');
        s.push_str(value);
        if let Some(seq) = word_info_seq_single(word_info) {
            s.push('\n');
            s.push_str(&get_senses_str(ctx, seq).await?);
        }
    } else {
        // dict.lisp:1765-1774
        let seq = word_info_seq_single(word_info);
        let conjs = word_info.conjugations.as_ref();
        // (cond ((and suffix (setf desc (get-suffix-description seq))) …)
        //       ((or (not conjs) (eql conjs :root)) …))
        let mut desc: Option<&'static str> = None;
        if suffix {
            if let Some(seq) = seq {
                desc = get_suffix_description(ctx, seq);
            }
        }
        if let Some(desc) = desc {
            write!(s, "  [suffix]: {} ", desc).unwrap();
        } else if conjs.is_none() || matches!(conjs, Some(WordConjugations::Root)) {
            s.push('\n');
            match seq {
                Some(seq) => s.push_str(&get_senses_str(ctx, seq).await?),
                None => s.push_str("???"),
            }
        }
        // (when seq (print-conj-info seq :out s :conjugations conjs))
        if let Some(seq) = seq {
            print_conj_info(ctx, seq, conjs, s).await?;
        }
    }
    Ok(())
}

// (word-info-seq word-info) is a single int or nil in the counter and default
// branches; a list seq only occurs on a compound/alternative word-info, which
// the components branch and top-level alternative loop handle first.
fn word_info_seq_single(word_info: &WordInfo) -> Option<i32> {
    match &word_info.seq {
        Some(WordInfoSeq::Single(seq)) => Some(*seq),
        None => None,
        Some(WordInfoSeq::Multi(_)) => {
            panic!("word-info-str: non-compound word-info seq is WordInfoSeq::Multi")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::word_info::{WordInfoKana, WordInfoType};
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn single(reading: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(reading.to_string()))
    }

    /// REPL fixtures (.103, `(word-info-str (make-instance 'word-info …))`),
    /// 2026-05-24, after `(init-suffixes t)`. Each row builds one word-info and
    /// pins the exact output (blank lines included). Covers:
    /// - A: default branch, no conjugations → senses.
    /// - B: default branch, conjugations nil → empty senses + full conj-info.
    /// - C: conjugations `:root` → conj display suppressed (test2 still fires).
    /// - D: seq nil → "???".
    /// - E: counter + seq → value then senses.
    /// - F: counter, no seq → value only.
    /// - G: compound, non-primary suffix component → marker, suffix description.
    /// - G2: compound, non-primary component without a suffix description →
    ///   marker, falls through to senses.
    /// - H: alternative → "<i>. " prefixes, second reading a counter.
    #[tokio::test]
    async fn word_info_str_fixtures() {
        use WordInfoType::{Kana, Kanji};
        let ctx = ctx_from_env().await;

        let compound = |text: &str, kana: &str, seqs: &[i32], comps: Vec<WordInfo>| WordInfo {
            kind: Kanji,
            text: text.to_string(),
            kana: single(kana),
            seq: Some(WordInfoSeq::Multi(
                seqs.iter().map(|s| Some(WordInfoSeq::Single(*s))).collect(),
            )),
            components: comps,
            ..Default::default()
        };

        let cases: Vec<(&str, WordInfo, &str)> = vec![
            (
                "A",
                WordInfo {
                    kind: Kanji,
                    text: "日本".to_string(),
                    kana: single("にほん"),
                    seq: Some(WordInfoSeq::Single(1582710)),
                    ..Default::default()
                },
                "日本 【にほん】\n1. [n] Japan",
            ),
            (
                "B",
                WordInfo {
                    kind: Kanji,
                    text: "食べた".to_string(),
                    kana: single("たべた"),
                    seq: Some(WordInfoSeq::Single(10092229)),
                    ..Default::default()
                },
                "食べた 【たべた】\n\n[ Conjugation: [v1] Past (~ta) Affirmative Plain\n  食べる 【たべる】 : to eat ]",
            ),
            (
                "C",
                WordInfo {
                    kind: Kanji,
                    text: "食べた".to_string(),
                    kana: single("たべた"),
                    seq: Some(WordInfoSeq::Single(10092229)),
                    conjugations: Some(WordConjugations::Root),
                    ..Default::default()
                },
                "食べた 【たべた】\n",
            ),
            (
                "D",
                WordInfo {
                    kind: Kana,
                    text: "ねこねこ".to_string(),
                    kana: single("ねこねこ"),
                    seq: None,
                    ..Default::default()
                },
                "ねこねこ\n???",
            ),
            (
                "E",
                WordInfo {
                    kind: Kanji,
                    text: "三冊".to_string(),
                    kana: single("さんさつ"),
                    seq: Some(WordInfoSeq::Single(1298520)),
                    counter: Some(("Value: 3".to_string(), false)),
                    ..Default::default()
                },
                "三冊 【さんさつ】\nValue: 3\n1. [ctr] counter for books\n2. [n] volume",
            ),
            (
                "F",
                WordInfo {
                    kind: Kanji,
                    text: "三".to_string(),
                    kana: single("さん"),
                    seq: None,
                    counter: Some(("Value: 3".to_string(), false)),
                    ..Default::default()
                },
                "三 【さん】\nValue: 3",
            ),
            (
                "G",
                compound(
                    "食べたい",
                    "たべたい",
                    &[1358280, 2017560],
                    vec![
                        WordInfo {
                            kind: Kanji,
                            text: "食べ".to_string(),
                            kana: single("たべ"),
                            seq: Some(WordInfoSeq::Single(1358280)),
                            primary: true,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kana,
                            text: "たい".to_string(),
                            kana: single("たい"),
                            seq: Some(WordInfoSeq::Single(2017560)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                ),
                "食べたい 【たべたい】 Compound word: 食べ + たい\n * 食べ 【たべ】\n1. [v1,vt] to eat\n2. [vt,v1] to live on (e.g. a salary); to live off; to subsist on\n * たい  [suffix]: want to... / would like to... ",
            ),
            (
                "G2",
                compound(
                    "日本語",
                    "にほんご",
                    &[1582710, 1576050],
                    vec![
                        WordInfo {
                            kind: Kanji,
                            text: "日本".to_string(),
                            kana: single("にほん"),
                            seq: Some(WordInfoSeq::Single(1582710)),
                            primary: true,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kanji,
                            text: "語".to_string(),
                            kana: single("ご"),
                            seq: Some(WordInfoSeq::Single(1576050)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                ),
                "日本語 【にほんご】 Compound word: 日本 + 語\n * 日本 【にほん】\n1. [n] Japan\n * 語 【ご】\n1. [adv,n] day before yesterday",
            ),
            (
                "H",
                WordInfo {
                    kind: Kanji,
                    text: "一人".to_string(),
                    kana: single("ひとり"),
                    seq: Some(WordInfoSeq::Multi(vec![
                        Some(WordInfoSeq::Single(1576150)),
                        Some(WordInfoSeq::Single(2149890)),
                    ])),
                    alternative: true,
                    components: vec![
                        WordInfo {
                            kind: Kanji,
                            text: "一人".to_string(),
                            kana: single("ひとり"),
                            seq: Some(WordInfoSeq::Single(1576150)),
                            primary: false,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kanji,
                            text: "一人".to_string(),
                            kana: single("ひとり"),
                            seq: Some(WordInfoSeq::Single(2149890)),
                            counter: Some(("Value: 1".to_string(), false)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                "<1>. 一人 【ひとり】\n1. [n] 《esp. 一人, １人》 one person\n2. [n] being alone; being by oneself\n3. [n] 《esp. 独り》 being single; being unmarried\n4. [adv] by oneself; alone\n5. [adv] 《with neg. sentence》 just; only; simply\n<2>. 一人 【ひとり】\nValue: 1\n1. [ctr] counter for people",
            ),
        ];

        for (label, word_info, expected) in &cases {
            assert_eq!(
                &word_info_str(&ctx, word_info).await.unwrap(),
                expected,
                "case={label}"
            );
        }
    }
}
}
