//! Port of `ichiran/dict:calc-score` (`dict.lisp:775`).
//!
//! Computes the segmenter's word-candidate score and the
//! [`KaniSegmentInfo`] property bag that the downstream scoring loop
//! (penalties, synergies, kanji-break) reads, dispatching on
//! compound-text, counter-text, and simple-text readings.

use std::borrow::Cow;

use crate::characters::char_class::CharClass;
use crate::characters::char_class::count_char_class;
use crate::characters::kana::mora_length;
use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_copulae_star_::COPULAE;
use crate::dict::_star_final_prt_star_::FINAL_PRT;
use crate::dict::_star_length_coeff_sequences_star_::KaniLengthClass;
use crate::dict::_star_non_final_prt_star_::NON_FINAL_PRT;
use crate::dict::_star_semi_final_prt_star_::semi_final_prt;
use crate::dict::_star_skip_words_star_::SKIP_WORDS;
use crate::dict::_star_weak_conj_forms_star_::WEAK_CONJ_FORMS;
use crate::dict::apply_score_mod::apply_score_mod;
use crate::dict::common::common as common_fn;
use crate::dict::compare_common::compare_common;
use crate::dict::compound_text_class::ScoreMod;
use crate::dict::conj_data_struct::ConjData;
use crate::dict::counter_text_class::Common;
use crate::dict::entry_dao::Entry;
use crate::dict::get_non_arch_posi::get_non_arch_posi;
use crate::dict::get_original_text::get_original_text;
use crate::dict::get_split::get_split;
use crate::dict::is_arch::is_arch as is_arch_fn;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::kanji_break_penalty::kanji_break_penalty;
use crate::dict::length_multiplier_coeff::length_multiplier_coeff;
use crate::dict::nokanji::nokanji;
use crate::dict::ord::ord as ord_fn;
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::score_base::score_base;
use crate::dict::segment_struct::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
use crate::dict::simple_text_class::{SimpleText, WordConjugations};
use crate::dict::skip_by_conj_data::skip_by_conj_data;
use crate::dict::test_conj_prop::test_conj_prop;
use crate::dict::text::text as text_fn;
use crate::dict::true_text::true_text;
use crate::dict::word_conj_data::word_conj_data;
use crate::dict::word_conjugations::word_conjugations;
use crate::dict::word_type::{word_type, WordType};

pub async fn calc_score(
    ctx: &KaniranContext,
    reading: &KaniWordDispatchEnum,
    final_: bool,
    use_length: Option<i32>,
    score_mod: Option<&ScoreMod>,
    kanji_break: &[usize],
) -> Result<(i32, Option<KaniSegmentInfo>), sqlx::Error> {
    // dict.lisp:780-792 (typecase reading (compound-text …) (counter-text (setf ctr-mode t)))
    if let KaniWordDispatchEnum::Compound(comp) = reading {
        // dict.lisp:782-784 — (args (list (score-base reading)
        //                                 :use-length (mora-length (text reading))
        //                                 :score-mod (score-mod reading)))
        let base: &KaniWordDispatchEnum = score_base(comp);
        let compound_text: Cow<'_, str> = text_fn(reading);
        let use_length_rec: Option<i32> = Some(mora_length(&compound_text) as i32);
        let score_mod_rec: Option<&ScoreMod> = Some(&comp.score_mod);

        // dict.lisp:785 (multiple-value-bind (score info) (apply 'calc-score args))
        let (mut rec_score, rec_info) = Box::pin(calc_score(
            ctx, base, false, use_length_rec, score_mod_rec, &[],
        ))
        .await?;

        // dict.lisp:785-786 — `(multiple-value-bind (score info) (apply 'calc-score args))`
        // followed by `(setf (getf info :conj) (word-conj-data reading))`.
        // When the inner call returns just `0` (one of the three skip
        // paths at dict.lisp:855-858), `info` binds to nil, and the
        // setf-place expansion of `(setf (getf nil :conj) X)` is CL's
        // setf-getf-on-nil idiom: it rewrites the binding to a fresh
        // plist `(:conj X)`. Mirror that by unwrapping into a zero/empty
        // KaniSegmentInfo whose `conj` field is then overwritten — the
        // five other fields are exactly the slots a one-key plist
        // leaves unset in the upstream representation. The synthesized
        // zero/empty defaults are inert under the cull invariant; see
        // the file-level doc-comment ("Structural divergence on the
        // compound synthesis path") for the per-key consumer audit
        // and the `dict-split.lisp:805` edge case to revisit when
        // `get-segsplit` is ported.
        let mut info = rec_info.unwrap_or_else(|| KaniSegmentInfo {
            posi: Vec::new(),
            seq_set: Vec::new(),
            conj: Vec::new(),
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: Vec::new(),
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        });
        info.conj = word_conj_data(ctx, reading).await?;

        // dict.lisp:787-788 (when kanji-break
        //                     (setf score (apply 'kanji-break-penalty kanji-break score
        //                                        :info info :text (text (car args)) (cdr args))))
        if !kanji_break.is_empty() {
            let base_text = text_fn(base);
            rec_score = Box::pin(kanji_break_penalty(
                ctx,
                kanji_break,
                rec_score,
                Some(&info),
                &base_text,
                use_length_rec,
                score_mod_rec,
            ))
            .await?;
        }
        return Ok((rec_score, Some(info)));
    }

    let ctr_mode = matches!(reading, KaniWordDispatchEnum::Counter(_));

    // dict.lisp:794-850 (let* (...) ...) — the main body's sequential bindings.
    let mut score: i32 = 1;
    let kanji_p = word_type(reading) == WordType::Kanji;
    let katakana_p = !kanji_p
        && count_char_class(&true_text(reading), CharClass::KatakanaUniq) > 0;
    let text: String = text_fn(reading).into_owned();
    let n_kanji = count_char_class(&text, CharClass::Kanji) as i32;
    let len: usize = mora_length(&text).max(1);

    // dict.lisp:801 (seq (the (or null fixnum) (seq reading)))
    //   — counter-text without source returns nil; simple-text always integer.
    let seq: Option<i32> = match reading {
        KaniWordDispatchEnum::Kanji(k) => Some(k.seq),
        KaniWordDispatchEnum::Kana(k) => Some(k.seq),
        KaniWordDispatchEnum::Proxy(p) => Some(p.source.seq()),
        KaniWordDispatchEnum::Counter(c) => match c.base().source.as_ref() {
            Some(crate::dict::counter_text_class::CounterSource::Kanji(k)) => Some(k.seq),
            Some(crate::dict::counter_text_class::CounterSource::Kana(k)) => Some(k.seq),
            None => None,
        },
        KaniWordDispatchEnum::Compound(_) => {
            unreachable!("compound-text returned early on the typecase branch")
        }
    };
    let mut ord: i32 = ord_fn(reading);

    // dict.lisp:803 (entry (and seq (get-dao 'entry seq)))
    let entry: Option<Entry> = match seq {
        Some(s) => {
            sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
                .bind(s)
                .fetch_optional(&ctx.pool)
                .await?
        }
        None => None,
    };

    // dict.lisp:804 (conj-only (let ((wc (word-conjugations reading))) (and wc (not (eql wc :root)))))
    let wc = word_conjugations(reading);
    let conj_only = matches!(wc, Some(WordConjugations::Ids(_)));

    // dict.lisp:805 (root-p (or ctr-mode (and (not conj-only) (root-p entry))))
    let root_p = ctr_mode
        || (!conj_only && entry.as_ref().map(|e| e.root_p).unwrap_or(false));

    // dict.lisp:806 (conj-data (word-conj-data reading))
    let mut conj_data: Vec<ConjData> = word_conj_data(ctx, reading).await?;

    // dict.lisp:808-811 (secondary-conj-p …) — `(or (every via …) (and (setf … delete-if …) nil))`
    let secondary_conj_p: bool = if conj_data.is_empty() {
        false
    } else if conj_data.iter().all(|cd| cd.via.is_some()) {
        true
    } else {
        conj_data.retain(|cd| cd.via.is_none());
        false
    };

    // dict.lisp:813-815 — conj-of / conj-props / conj-types
    //
    // `cd.prop` is `Option<ConjProp>` to mirror the Lisp `defstruct`'s
    // permissive nil default, but the only producer (`get_conj_data`)
    // always emits `Some(prop)`. Upstream `(mapcar 'conj-type conj-props)`
    // would error on a nil prop because `conj-type` is a struct slot
    // reader; mirror that by panicking via `.expect(...)` rather than
    // silently dropping a nil-prop entry (which would also drop the
    // matching `conj-of` value out of sync).
    let conj_of: Vec<i32> = conj_data.iter().filter_map(|cd| cd.from).collect();
    let conj_props: Vec<&crate::dict::conj_prop_dao::ConjProp> = conj_data
        .iter()
        .map(|cd| {
            cd.prop
                .as_ref()
                .expect("conj-data.prop is populated by get-conj-data")
        })
        .collect();
    let conj_types: Vec<i32> = conj_props.iter().map(|p| p.conj_type).collect();

    // dict.lisp:816-819 (conj-types-p …)
    let conj_types_p = root_p
        || use_length.is_some()
        || !conj_props
            .iter()
            .all(|p| test_conj_prop(p, WEAK_CONJ_FORMS));

    // dict.lisp:820 (seq-set (and seq (cons seq conj-of)))
    let seq_set: Vec<i32> = match seq {
        Some(s) => {
            let mut v = Vec::with_capacity(1 + conj_of.len());
            v.push(s);
            v.extend(conj_of.iter().copied());
            v
        }
        None => Vec::new(),
    };

    // dict.lisp:821 (sp-seq-set (if (and seq root-p (not use-length)) (list seq) seq-set))
    let sp_seq_set: Vec<i32> = if seq.is_some() && root_p && use_length.is_none() {
        vec![seq.unwrap()]
    } else {
        seq_set.clone()
    };

    // dict.lisp:822-824 (prefer-kana (select-dao 'sense-prop (:and (:in 'seq …) (:= 'tag "misc") (:= 'text "uk"))))
    let prefer_kana_sense_ids: Vec<i32> = if sp_seq_set.is_empty() {
        Vec::new()
    } else {
        sqlx::query_scalar(
            "SELECT sense_id FROM sense_prop \
             WHERE seq = ANY($1) AND tag = 'misc' AND text = 'uk'",
        )
        .bind(&sp_seq_set)
        .fetch_all(&ctx.pool)
        .await?
    };
    let prefer_kana = !prefer_kana_sense_ids.is_empty();

    // dict.lisp:825 (is-arch (every 'is-arch sp-seq-set))
    let is_arch = sp_seq_set.iter().all(|s| is_arch_fn(ctx, *s));

    // dict.lisp:826-827 (posi (if ctr-mode (list "ctr") (get-non-arch-posi seq-set)))
    let posi: Vec<String> = if ctr_mode {
        vec!["ctr".to_string()]
    } else if seq_set.is_empty() {
        Vec::new()
    } else {
        get_non_arch_posi(ctx, &seq_set).await?
    };

    // dict.lisp:828-830 (common / common-of / common-p)
    let initial_common = if conj_only {
        Common::Null
    } else {
        common_fn(reading)
    };
    let mut common_value: i32 = match initial_common {
        Common::Score(n) => n,
        _ => 0,
    };
    let mut common_p: bool = matches!(initial_common, Common::Score(_));
    let mut common_of: Option<i32> = if common_p { Some(common_value) } else { None };

    // dict.lisp:831-835
    let particle_p = posi.iter().any(|s| s == "prt");
    let semi_final_particle_p =
        seq.is_some_and(|s| semi_final_prt().contains(&s));
    let non_final_particle_p =
        seq.is_some_and(|s| NON_FINAL_PRT.contains(&s));
    let pronoun_p = posi.iter().any(|s| s == "pn");
    let cop_da_p = seq_set.iter().any(|s| COPULAE.contains(s));

    // dict.lisp:836-844 (long-p …)
    let long_threshold: usize = if kanji_p
        && !prefer_kana
        && ((root_p && conj_data.is_empty())
            || (use_length.is_some() && conj_types.contains(&13)))
    {
        2
    } else if common_p && common_value > 0 && common_value < 10 {
        2
    } else if (conj_types.contains(&3) || conj_types.contains(&9))
        && use_length.is_none()
    {
        4
    } else {
        3
    };
    let long_p = len > long_threshold;

    // dict.lisp:845-847 (no-common-bonus …)
    let no_common_bonus = particle_p
        || !conj_types_p
        || (!long_p && posi.len() == 1 && posi[0] == "int");

    let mut primary_p = false;
    let mut use_length_bonus: i32 = 0;
    let mut split_info: KaniSplitInfo = KaniSplitInfo::None;

    // dict.lisp:855-858 (when (or (intersection seq-set *skip-words*)
    //                              (and (not final) (member seq *final-prt*))
    //                              (and (not root-p) (skip-by-conj-data conj-data)))
    //                     (return-from calc-score 0))
    if seq_set.iter().any(|s| SKIP_WORDS.contains(s))
        || (!final_ && seq.is_some_and(|s| FINAL_PRT.contains(&s)))
        || (!root_p && skip_by_conj_data(&conj_data))
    {
        return Ok((0, None));
    }

    // dict.lisp:859-870 (when (and conj-data (not (and (= ord 0) common-p))) …)
    if !conj_data.is_empty() && !(ord == 0 && common_p) {
        // dict.lisp:860 (get-original-text reading :conj-data conj-data)
        let reading_simple: KaniSimpleTextDispatchEnum = match reading {
            KaniWordDispatchEnum::Kanji(k) => KaniSimpleTextDispatchEnum::Kanji(k.clone()),
            KaniWordDispatchEnum::Kana(k) => KaniSimpleTextDispatchEnum::Kana(k.clone()),
            KaniWordDispatchEnum::Proxy(p) => KaniSimpleTextDispatchEnum::Proxy(p.clone()),
            // dict.lisp:780-792 — counter-text has empty conj-data and compound-text
            // returned early; only the simple-text family reaches here.
            KaniWordDispatchEnum::Counter(_) | KaniWordDispatchEnum::Compound(_) => {
                unreachable!("conj-data block reached for non-simple-text reading")
            }
        };
        let orig_texts =
            get_original_text(ctx, &reading_simple, Some(&conj_data)).await?;

        // dict.lisp:860-861 collect (list (common ot) (ord ot))
        let conj_of_data: Vec<(Option<i32>, i32)> = orig_texts
            .iter()
            .map(|ot| match ot {
                KaniSimpleTextDispatchEnum::Kanji(k) => (k.common, k.ord),
                KaniSimpleTextDispatchEnum::Kana(k) => (k.common, k.ord),
                KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!(
                    "get-original-text returns kanji/kana variants only"
                ),
            })
            .collect();

        if !conj_of_data.is_empty() {
            // dict.lisp:863-867 (unless common-p …) — only collect/sort conj-of-common
            // when the candidate doesn't already have a common rank.
            if !common_p {
                let conj_of_common: Vec<i32> = conj_of_data
                    .iter()
                    .filter_map(|(c, _)| *c)
                    .collect();
                if !conj_of_common.is_empty() {
                    // dict.lisp:867 (car (sort conj-of-common #'compare-common))
                    // `compare-common` is a binary predicate (not a strict-weak
                    // ordering), so it's not safe to drive Rust's `sort_by` with
                    // it — only the single-minimum lookup the upstream `(car …)`
                    // expresses is well-defined. `min_by` is enough.
                    common_of = conj_of_common.iter().copied().min_by(|a, b| {
                        if compare_common(Some(*a as i64), Some(*b as i64))
                            .is_truthy()
                        {
                            std::cmp::Ordering::Less
                        } else {
                            std::cmp::Ordering::Greater
                        }
                    });
                    common_value = 0;
                    common_p = true;
                }
            }
            // dict.lisp:868-870 (let ((conj-of-ord (reduce 'min conj-of-data :key 'second)))
            //                     (when (< conj-of-ord ord) (setf ord conj-of-ord)))
            let conj_of_ord = conj_of_data.iter().map(|(_, o)| *o).min().unwrap();
            if conj_of_ord < ord {
                ord = conj_of_ord;
            }
        }
    }

    // dict.lisp:872-888 (unless is-arch (setf primary-p …))
    //
    // The upstream `(or cond_a cond_b cond_c cond_d)` short-circuits
    // pairwise, so `cond_b`/`cond_c`'s entry-slot reads are only reached
    // when `cond_a` (entry is nil) is false. Mirror that in Rust by
    // matching on `entry.as_ref()`: the `None` arm IS `cond_a` true,
    // and the `Some(e)` arm evaluates the remaining conditions over the
    // unwrapped entry. (`cond_d` doesn't read entry, so it falls inside
    // the `Some` arm at the same nesting depth.)
    if !is_arch {
        primary_p = match entry.as_ref() {
            None => true,
            Some(e) => {
                // dict.lisp:875-878 (and prefer-kana conj-types-p (not kanji-p)
                //                       (or (not (primary-nokanji entry)) (nokanji reading)))
                let cond_b = prefer_kana
                    && conj_types_p
                    && !kanji_p
                    && (!e.primary_nokanji || nokanji(reading).unwrap_or(false));
                // dict.lisp:879-883 (and (or (= ord 0) cop-da-p) (or kanji-p conj-types-p) (...))
                let cond_c = (ord == 0 || cop_da_p)
                    && (kanji_p || conj_types_p)
                    && ((kanji_p && !prefer_kana)
                        || (common_p && pronoun_p)
                        || e.n_kanji == 0);
                // dict.lisp:884-887 (and prefer-kana kanji-p (= ord 0)
                //                       (not (query (:select 'id :from 'sense
                //                                            :where (:and (:in 'id (:set …)) (:= 'ord 0))))))
                //
                // Inert divergence: PG `WHERE id = ANY($1)` returns zero
                // rows for an empty array, whereas upstream's
                // `(:in 'id (:set …))` would emit `IN ()` and crash on an
                // empty `prefer_kana_sense_ids`. Both are unreachable
                // here: `prefer_kana` is `true` iff the sense-prop SELECT
                // above returned at least one row, which is what builds
                // `prefer_kana_sense_ids` non-empty. Don't widen the gate
                // without re-checking this invariant.
                let cond_d = if prefer_kana && kanji_p && ord == 0 {
                    let any_sense_ord_zero: Option<i32> = sqlx::query_scalar(
                        "SELECT id FROM sense WHERE id = ANY($1) AND ord = 0",
                    )
                    .bind(&prefer_kana_sense_ids)
                    .fetch_optional(&ctx.pool)
                    .await?;
                    any_sense_ord_zero.is_none()
                } else {
                    false
                };
                cond_b || cond_c || cond_d
            }
        };
    }

    // dict.lisp:890-895 (when primary-p (incf score …))
    if primary_p {
        let bump = if long_p {
            10
        } else if secondary_conj_p && !kanji_p {
            2
        } else if common_p && conj_types_p {
            5
        } else if prefer_kana
            || entry.is_none()
            || entry.as_ref().map(|e| e.n_kanji == 0).unwrap_or(false)
        {
            3
        } else {
            2
        };
        score += bump;
    }

    // dict.lisp:896-902 (when (and particle-p (or final (not semi-final-particle-p))) …)
    if particle_p && (final_ || !semi_final_particle_p) {
        score += 2;
        if common_p {
            score += 2 + len as i32;
        }
        if final_ && !non_final_particle_p {
            if primary_p {
                score += 5;
            } else if semi_final_particle_p {
                score += 2;
            }
        }
    }

    // dict.lisp:903-918 (when (and common-p (not no-common-bonus)) …)
    if common_p && !no_common_bonus {
        let mut common_bonus: i32 = if secondary_conj_p && use_length.is_none() {
            if kanji_p && primary_p { 4 } else { 2 }
        } else if long_p
            || cop_da_p
            || (root_p && (kanji_p || (primary_p && len > 2)))
        {
            if common_value == 0 {
                10
            } else if !primary_p {
                (15 - common_value).max(10)
            } else {
                (20 - common_value).max(10)
            }
        } else if kanji_p {
            8
        } else if primary_p {
            4
        } else if len > 2 || (common_value > 0 && common_value < 10) {
            3
        } else {
            2
        };
        // dict.lisp:916-917 (when (and (>= common-bonus 10) (find 10 conj-types))
        //                     (decf common-bonus 4))
        if common_bonus >= 10 && conj_types.contains(&10) {
            common_bonus -= 4;
        }
        score += common_bonus;
    }

    // dict.lisp:919-920 (when long-p (setf score (max len score)))
    if long_p {
        score = (len as i32).max(score);
    }
    // dict.lisp:921-924 (when kanji-p ...)
    if kanji_p {
        score = (if is_arch { 3 } else { 5 }).max(score);
        if long_p && (n_kanji > 1 || len > 4) {
            score += 2;
        }
    }
    // dict.lisp:925-926 (when ctr-mode (setf score (max 5 score)))
    if ctr_mode {
        score = 5.max(score);
    }

    // dict.lisp:927-929
    let prop_score: i32 = score;
    let multiplier_class = if kanji_p || katakana_p {
        KaniLengthClass::Strong
    } else {
        KaniLengthClass::Weak
    };
    let multiplier = length_multiplier_coeff(len as i64, multiplier_class) as i32;
    let kanji_bonus: i32 = if n_kanji > 1 { (n_kanji - 1) * 5 } else { 0 };
    score = prop_score * (multiplier + kanji_bonus);

    // dict.lisp:931-937 (when use-length …)
    if let Some(ul) = use_length {
        let delta: i64 = (ul - len as i32) as i64;
        let tail_class = if len > 3 && (kanji_p || katakana_p) {
            KaniLengthClass::Ltail
        } else {
            KaniLengthClass::Tail
        };
        use_length_bonus += prop_score * (length_multiplier_coeff(delta, tail_class) as i32);
        if let Some(sm) = score_mod {
            use_length_bonus += apply_score_mod(sm, prop_score as i64, delta) as i32;
        }
        score += use_length_bonus;
    }

    // dict.lisp:939-974 (unless ctr-mode (multiple-value-bind (split score-mod-split) …))
    let mut prop_score = prop_score;
    if !ctr_mode {
        // dict.lisp:940 (get-split reading conj-of) — reading is simple-text here.
        let reading_simple: KaniSimpleTextDispatchEnum = match reading {
            KaniWordDispatchEnum::Kanji(k) => KaniSimpleTextDispatchEnum::Kanji(k.clone()),
            KaniWordDispatchEnum::Kana(k) => KaniSimpleTextDispatchEnum::Kana(k.clone()),
            KaniWordDispatchEnum::Proxy(p) => KaniSimpleTextDispatchEnum::Proxy(p.clone()),
            KaniWordDispatchEnum::Counter(_) | KaniWordDispatchEnum::Compound(_) => {
                unreachable!("split branch only reached for simple-text")
            }
        };
        if let Some((parts, score_mod_split)) = get_split(ctx, &reading_simple, &conj_of).await? {
            // dict.lisp:943-945 ((member :score split) …)
            if parts.iter().any(|p| matches!(p, SplitPart::Score)) {
                score += score_mod_split;
                split_info = KaniSplitInfo::Score(score_mod_split);
            }
            // dict.lisp:946-949 ((member :pscore split) …)
            else if parts.iter().any(|p| matches!(p, SplitPart::PScore)) {
                let new_prop_score = 1.max(prop_score + score_mod_split);
                // dict.lisp:948 (ceiling (* score new-prop-score) prop-score)
                score = ceiling_div(score * new_prop_score, prop_score);
                prop_score = new_prop_score;
            }
            // dict.lisp:950-974 (split …) — every element is a Word here.
            else {
                let words: Vec<&KaniWordDispatchEnum> = parts
                    .iter()
                    .map(|p| match p {
                        SplitPart::Word(w) => w,
                        _ => unreachable!("Score/PScore caught by earlier branches"),
                    })
                    .collect();
                let nparts = words.len();
                let outer_text_chars: usize = text.chars().count();

                let mut slen: usize = 0;
                let mut smlen: usize = 0;
                let mut part_scores: Vec<i32> = Vec::with_capacity(nparts);

                for (cnt0, &part) in words.iter().enumerate() {
                    let cnt = cnt0 + 1;
                    let last = cnt == nparts;
                    let ptext: Cow<'_, str> = text_fn(part);
                    let plen: usize = ptext.chars().count();
                    slen += plen;
                    let pmlen: usize = mora_length(&ptext);
                    smlen += pmlen;

                    // dict.lisp:962-965 (if (and last (> slen (length text)))
                    //                     (make-instance 'proxy-text :source part :text ... :kana ""))
                    let tpart: KaniWordDispatchEnum = if last && slen > outer_text_chars {
                        let new_len: usize = (1i32)
                            .max(plen as i32 + outer_text_chars as i32 - slen as i32)
                            as usize;
                        let truncated_text: String =
                            ptext.chars().take(new_len).collect();
                        let part_simple: KaniSimpleTextDispatchEnum = match part {
                            KaniWordDispatchEnum::Kanji(k) => {
                                KaniSimpleTextDispatchEnum::Kanji(k.clone())
                            }
                            KaniWordDispatchEnum::Kana(k) => {
                                KaniSimpleTextDispatchEnum::Kana(k.clone())
                            }
                            KaniWordDispatchEnum::Proxy(p) => {
                                KaniSimpleTextDispatchEnum::Proxy(p.clone())
                            }
                            KaniWordDispatchEnum::Counter(_)
                            | KaniWordDispatchEnum::Compound(_) => unreachable!(
                                "split parts come from find-word-{{seq,conj-of}}; \
                                 simple-text only"
                            ),
                        };
                        KaniWordDispatchEnum::Proxy(ProxyText {
                            text: truncated_text,
                            kana: String::new(),
                            source: Box::new(part_simple),
                            state: SimpleText::default(),
                        })
                    } else {
                        part.clone()
                    };

                    // dict.lisp:966-970
                    let part_use_length: Option<i32> = if last {
                        use_length.map(|ul| pmlen as i32 + ul - smlen as i32)
                    } else {
                        None
                    };
                    // dict.lisp:970 (if last score-mod 0) — pass score_mod when last, else default.
                    let part_score_mod: Option<&ScoreMod> = if last { score_mod } else { None };

                    let (part_score, _info) = Box::pin(calc_score(
                        ctx,
                        &tpart,
                        final_ && last,
                        part_use_length,
                        part_score_mod,
                        &[],
                    ))
                    .await?;
                    part_scores.push(part_score);
                }

                // dict.lisp:973-974 (setf split-info (cons score-mod-split part-scores))
                //                   (return (reduce '+ part-scores))
                let sum: i32 = part_scores.iter().sum();
                score = score_mod_split + sum;
                split_info = KaniSplitInfo::Parts {
                    score_mod: score_mod_split,
                    part_scores,
                };
            }
        }
    }

    // dict.lisp:976-980 — final info construction
    let info = KaniSegmentInfo {
        posi: posi.clone(),
        seq_set: seq_set.clone(),
        conj: conj_data.clone(),
        common: if common_p { common_of } else { None },
        score_info: KaniScoreInfo {
            prop_score,
            kanji_break: kanji_break.to_vec(),
            use_length_bonus,
            split_info,
        },
        kpcl: (kanji_p || katakana_p, primary_p, common_p, long_p),
    };

    // dict.lisp:981-982 (when kanji-break …)
    let final_score = if !kanji_break.is_empty() {
        Box::pin(kanji_break_penalty(
            ctx,
            kanji_break,
            score,
            Some(&info),
            &text,
            use_length,
            score_mod,
        ))
        .await?
    } else {
        score
    };
    Ok((final_score, Some(info)))
}

/// `(ceiling a b)` first return value for positive `a` and positive `b`.
/// CL ceiling rounds the quotient toward positive infinity; the Rust
/// expression `(a + b - 1) / b` matches for non-negative operands.
/// Inlined here rather than reaching for an unstable `i32::div_ceil`
/// so the port compiles on stable.
fn ceiling_div(a: i32, b: i32) -> i32 {
    (a + b - 1) / b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::segment_struct::KaniSplitInfo;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_word_for(ctx: &KaniranContext, s: &str) -> KaniWordDispatchEnum {
        match find_word(ctx, s, false).await.unwrap() {
            FindWordRows::Kana(mut v) => KaniWordDispatchEnum::Kana(v.remove(0)),
            FindWordRows::Kanji(mut v) => KaniWordDispatchEnum::Kanji(v.remove(0)),
        }
    }

    /// Fetch a specific kana_text row by (seq, text) — deterministic
    /// alternative to `first_word_for` when find-word's row order
    /// (no upstream ORDER BY) would make a test flaky.
    async fn kana_by_seq_text(
        ctx: &KaniranContext,
        seq: i32,
        text: &str,
    ) -> KaniWordDispatchEnum {
        let rows: Vec<crate::dict::kana_text_dao::KanaText> = sqlx::query_as(
            "SELECT * FROM kana_text WHERE seq = $1 AND text = $2 ORDER BY id LIMIT 1",
        )
        .bind(seq)
        .bind(text)
        .fetch_all(&ctx.pool)
        .await
        .expect("query");
        KaniWordDispatchEnum::Kana(rows.into_iter().next().expect("row exists"))
    }

    // ----- REPL-pinned cases (.103, 2026-05-16). All output strings
    //       captured by `(calc-score (car (find-word "<txt>")) …)`. -----

    /// REPL: `(calc-score (car (find-word "ねこ")))` →
    ///   `16  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7 :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))`
    #[tokio::test]
    async fn nekko_baseline() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "ねこ").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1467640]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(7));
        assert_eq!(info.score_info.prop_score, 4);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, true, false));
    }

    /// REPL: `(calc-score (car (find-word "の")))` →
    ///   `11  (:POSI ("prt") :SEQ-SET (1469800) :CONJ NIL :COMMON 0 :SCORE-INFO (11 NIL 0 NIL) :KPCL (NIL T T NIL))`
    #[tokio::test]
    async fn no_particle_non_final() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "の").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 11);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["prt".to_string()]);
        assert_eq!(info.seq_set, vec![1469800]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 11);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    /// REPL: `(calc-score (car (find-word "の")) :final t)` →
    ///   `16  (:POSI ("prt") :SEQ-SET (1469800) :CONJ NIL :COMMON 0 :SCORE-INFO (16 NIL 0 NIL) :KPCL (NIL T T NIL))`
    #[tokio::test]
    async fn no_particle_final_branch_bonus() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "の").await;
        let (score, info) = calc_score(&ctx, &w, true, None, None, &[]).await.unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["prt".to_string()]);
        assert_eq!(info.seq_set, vec![1469800]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    /// REPL: `(calc-score (car (find-word "は")))` →
    ///   `1  (:POSI ("n") :SEQ-SET (1171680) :CONJ NIL :COMMON NIL :SCORE-INFO (1 NIL 0 NIL) :KPCL (NIL NIL NIL NIL))`
    #[tokio::test]
    async fn ha_n_uncommon() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "は").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 1);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1171680]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 1);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, false, false));
    }

    /// REPL: with row `(select-dao 'kana-text (:and (:= 'seq 2089020) (:= 'text "だ")))` →
    ///   `(calc-score row)` and `(calc-score row :final t)` both →
    ///   `16  (:POSI ("aux-v" "cop" "cop-da") :SEQ-SET (2089020) :CONJ NIL :COMMON 0
    ///         :SCORE-INFO (16 NIL 0 NIL) :KPCL (NIL T T NIL))`
    ///
    /// Pinned via the explicit-seq helper because `(find-word "だ")`
    /// returns 6 candidates and Postgres's row order without an
    /// ORDER BY is non-deterministic — `da_first_reading_n` would
    /// otherwise alternate between the `n` reading (seq 1564200) and
    /// the copula (seq 2089020). This case exercises the `cop-da-p`
    /// branch (`(intersection seq-set *copulae*)` truthy) and the
    /// `common = 0` arm of the common-bonus cascade.
    #[tokio::test]
    async fn da_copula_cop_da_p_branch() {
        let ctx = ctx_from_env().await;
        let w = kana_by_seq_text(&ctx, 2089020, "だ").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        let mut posi_sorted = info.posi.clone();
        posi_sorted.sort();
        assert_eq!(
            posi_sorted,
            vec!["aux-v".to_string(), "cop".to_string(), "cop-da".to_string()]
        );
        assert_eq!(info.seq_set, vec![2089020]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));

        // :final t produces the same value — particle_p is false (no "prt" in posi),
        // so the final-particle bonus block is skipped.
        let (score, _) = calc_score(&ctx, &w, true, None, None, &[]).await.unwrap();
        assert_eq!(score, 16);
    }

    /// REPL: `(calc-score (car (find-word "食べる")))` →
    ///   `504  (:POSI ("v1" "vt") :SEQ-SET (1358280) :CONJ NIL :COMMON 25 :SCORE-INFO (21 NIL 0 NIL) :KPCL (T T T T))`
    #[tokio::test]
    async fn taberu_root_verb() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "食べる").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 504);
        let info = info.unwrap();
        let mut p = info.posi.clone();
        p.sort();
        assert_eq!(p, vec!["v1".to_string(), "vt".to_string()]);
        assert_eq!(info.seq_set, vec![1358280]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(25));
        assert_eq!(info.score_info.prop_score, 21);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, true));
    }

    /// REPL: `(calc-score (car (find-word "食べる")) :kanji-break '(0))` →
    ///   `252  (... :SCORE-INFO (21 (0) 0 NIL) ...)`
    /// `kanji_break_penalty` only adjusts the outer score; it does not
    /// mutate any field of `info`. Every non-score-info field is
    /// therefore identical to [`taberu_root_verb`]'s output.
    #[tokio::test]
    async fn taberu_with_kanji_break() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "食べる").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[0]).await.unwrap();
        assert_eq!(score, 252);
        let info = info.unwrap();
        let mut p = info.posi.clone();
        p.sort();
        assert_eq!(p, vec!["v1".to_string(), "vt".to_string()]);
        assert_eq!(info.seq_set, vec![1358280]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(25));
        assert_eq!(info.score_info.prop_score, 21);
        assert_eq!(info.score_info.kanji_break, vec![0]);
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, true));
    }

    /// REPL: `(calc-score (car (find-word "ありがとう")))` →
    ///   `525  (:POSI ("int") :SEQ-SET (1586820) :CONJ NIL :COMMON 0 :SCORE-INFO (21 NIL 0 NIL) :KPCL (NIL T T T))`
    #[tokio::test]
    async fn arigatou_interjection_long() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "ありがとう").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 525);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["int".to_string()]);
        assert_eq!(info.seq_set, vec![1586820]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 21);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, true));
    }

    /// REPL: `(calc-score (car (find-word "コンピューター")))` →
    ///   `440  (:POSI ("n") :SEQ-SET (1053350) :CONJ NIL :COMMON 0 :SCORE-INFO (11 NIL 0 NIL) :KPCL (T NIL T T))`
    #[tokio::test]
    async fn computer_katakana_path() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "コンピューター").await;
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 440);
        let info = info.unwrap();
        assert_eq!(info.posi, vec!["n".to_string()]);
        assert_eq!(info.seq_set, vec![1053350]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 11);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        // kpcl: katakana-p=T, primary-p=NIL, common-p=T, long-p=T
        assert_eq!(info.kpcl, (true, false, true, true));
    }

    /// REPL: `(calc-score (car (find-word "ねこ")) :use-length 5)` →
    ///   `80  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///        :SCORE-INFO (4 NIL 64 NIL) :KPCL (NIL NIL T NIL))`
    /// REPL: `(calc-score (car (find-word "ねこ")) :use-length 3)` →
    ///   `32  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///        :SCORE-INFO (4 NIL 16 NIL) :KPCL (NIL NIL T NIL))`
    /// REPL: `(calc-score (car (find-word "ねこ")) :use-length 2)` →
    ///   `16  (:POSI ("n") :SEQ-SET (1467640) :CONJ NIL :COMMON 7
    ///        :SCORE-INFO (4 NIL 0 NIL) :KPCL (NIL NIL T NIL))`
    #[tokio::test]
    async fn neko_use_length_variations() {
        let ctx = ctx_from_env().await;
        let w = first_word_for(&ctx, "ねこ").await;

        // Helper to assert every field other than score / use_length_bonus,
        // which are the only quantities that change with use_length.
        let assert_neko_baseline = |info: &KaniSegmentInfo| {
            assert_eq!(info.posi, vec!["n".to_string()]);
            assert_eq!(info.seq_set, vec![1467640]);
            assert!(info.conj.is_empty());
            assert_eq!(info.common, Some(7));
            assert_eq!(info.score_info.prop_score, 4);
            assert!(info.score_info.kanji_break.is_empty());
            assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
            assert_eq!(info.kpcl, (false, false, true, false));
        };

        let (score, info) = calc_score(&ctx, &w, false, Some(5), None, &[]).await.unwrap();
        assert_eq!(score, 80);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 64);

        let (score, info) = calc_score(&ctx, &w, false, Some(3), None, &[]).await.unwrap();
        assert_eq!(score, 32);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 16);

        let (score, info) = calc_score(&ctx, &w, false, Some(2), None, &[]).await.unwrap();
        assert_eq!(score, 16);
        let info = info.unwrap();
        assert_neko_baseline(&info);
        assert_eq!(info.score_info.use_length_bonus, 0);
    }

    // ----- Class-C regression: compound-text whose inner calc-score
    //       on `score-base` hits a skip-path and returns `0` with no
    //       info. Upstream `dict.lisp:785-786`:
    //           (multiple-value-bind (score info) (apply 'calc-score args)
    //             (setf (getf info :conj) (word-conj-data reading)) ...)
    //       The `multiple-value-bind` of a single-value return binds
    //       `info` to nil; `(setf (getf nil :conj) X)` is CL's
    //       setf-getf-on-nil idiom, which rewrites the binding to a
    //       fresh plist `(:conj X)`. The Rust port must mirror this
    //       rather than propagating `(0, None)`.
    //
    //       Both rows below come from the chunk_b_segmentation_2026_05_14
    //       parquet — the captured args / result tell us exactly what
    //       outer info plist upstream produced. The current `None =>`
    //       arm at line 167 returns `(0, None)`, so both tests fail
    //       on `info.expect(...)`.

    use crate::dict::compound_text_class::CompoundText;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::simple_text_class::SimpleText;

    /// Row 279 of chunk_b calc_score parquet. Compound `れちゃう`,
    /// `score_base = null` (falls back to primary), `score_mod = 5`.
    /// Last word `ちゃう` (seq 2013800, conjugations=:ROOT) — and
    /// `get-conj-data(2013800, :ROOT, "ちゃう")` returns nil on the
    /// real DB, so the outer `(word-conj-data <compound>)` returns nil
    /// and the synthesized plist has `:CONJ null`.
    ///
    /// Captured:
    ///   args   = [<compound>, ":FINAL", null, ":KANJI-BREAK", null]
    ///   result = [0, [":CONJ", null]]
    #[tokio::test]
    async fn compound_skipword_partial_info_conj_null() {
        let ctx = ctx_from_env().await;

        let primary = KanaText {
            id: 532088,
            seq: 10230770,
            text: "れて".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![232360])),
                hintedp: false,
            },
        };
        let tail = KanaText {
            id: 108760,
            seq: 2013800,
            text: "ちゃう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Root),
                hintedp: false,
            },
        };
        let compound = CompoundText {
            text: "れちゃう".into(),
            kana: "れちゃう".into(),
            primary: Box::new(KaniWordDispatchEnum::Kana(primary.clone())),
            words: vec![
                KaniWordDispatchEnum::Kana(primary),
                KaniWordDispatchEnum::Kana(tail),
            ],
            score_base: None,
            score_mod: ScoreMod::Single(5),
        };
        let word = KaniWordDispatchEnum::Compound(compound);

        let (score, info) = calc_score(&ctx, &word, false, None, None, &[])
            .await
            .unwrap();
        assert_eq!(score, 0);
        let info = info.expect(
            "compound + skip-word inner must synthesize partial info \
             (mirrors upstream `(setf (getf nil :conj) X)`)",
        );
        assert!(
            info.conj.is_empty(),
            "info.conj: expected empty (last word ちゃう :ROOT has no conj-data); got {:?}",
            info.conj,
        );
        // The other five fields default to zero/empty — only `:CONJ`
        // is set by the setf-getf-on-nil synthesis.
        assert!(info.posi.is_empty(), "info.posi: {:?}", info.posi);
        assert!(info.seq_set.is_empty(), "info.seq_set: {:?}", info.seq_set);
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 0);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, false, false));
    }

    /// chunk_b calc_score row capturing compound `られなくなりました`,
    /// `score_base = null`, `score_mod = (1 5)` (list). Last word
    /// `なりました` (seq 10374833, conjugations=[378046]) — the outer
    /// `(word-conj-data <compound>)` recurses to it and produces a
    /// single CONJ-DATA: seq=10374833, from=1375610 (なる), conj-prop
    /// (id=386572, conj_id=378046, conj_type=2, fml=true, pos="v5r"),
    /// src_map=[("なりました","なる")].
    ///
    /// Captured:
    ///   result = [0, [":CONJ", [{class:CONJ-DATA, seq:10374833,
    ///            from:1375610, prop:{conj_id:378046, conj_type:2,
    ///            fml:true, neg:null, pos:"v5r"}, src_map:[[なりました,
    ///            なる]]}]]]
    #[tokio::test]
    async fn compound_skipword_partial_info_conj_non_null() {
        let ctx = ctx_from_env().await;

        let rare = KanaText {
            id: 0,
            seq: 10230810,
            text: "られ".into(),
            ord: 1,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![232400])),
                hintedp: false,
            },
        };
        let naku = KanaText {
            id: 1030305,
            seq: 10648808,
            text: "なく".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![656991])),
                hintedp: false,
            },
        };
        let narimashita = KanaText {
            id: 704041,
            seq: 10374833,
            text: "なりました".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(vec![378046])),
                hintedp: false,
            },
        };
        let compound = CompoundText {
            text: "られなくなりました".into(),
            kana: "られなくなりました".into(),
            primary: Box::new(KaniWordDispatchEnum::Kana(rare.clone())),
            words: vec![
                KaniWordDispatchEnum::Kana(rare),
                KaniWordDispatchEnum::Kana(naku),
                KaniWordDispatchEnum::Kana(narimashita),
            ],
            score_base: None,
            score_mod: ScoreMod::Stack(vec![ScoreMod::Single(1), ScoreMod::Single(5)]),
        };
        let word = KaniWordDispatchEnum::Compound(compound);

        let (score, info) = calc_score(&ctx, &word, false, None, None, &[])
            .await
            .unwrap();
        assert_eq!(score, 0);
        let info = info.expect(
            "compound + skip-word inner must synthesize partial info \
             carrying the outer word_conj_data",
        );
        assert_eq!(
            info.conj.len(),
            1,
            "info.conj: expected single CONJ-DATA from last word なりました; got {:?}",
            info.conj,
        );
        let cd = &info.conj[0];
        assert_eq!(cd.seq, Some(10374833));
        assert_eq!(cd.from, Some(1375610));
        assert_eq!(cd.via, None);
        let prop = cd.prop.as_ref().expect("conj-prop present");
        assert_eq!(prop.id, 386572);
        assert_eq!(prop.conj_id, 378046);
        assert_eq!(prop.conj_type, 2);
        assert_eq!(prop.pos, "v5r");
        // REPL: SELECT neg, fml FROM conj_prop WHERE id = 386572; → f, t
        // PG false (`f`) decodes to Some(false) at the sqlx layer.
        assert_eq!(prop.neg, Some(false));
        assert_eq!(prop.fml, Some(true));
        assert_eq!(
            cd.src_map,
            vec![("なりました".to_string(), "なる".to_string())],
        );
        // Same zero/empty defaults for the five unsynthesized fields.
        assert!(info.posi.is_empty());
        assert!(info.seq_set.is_empty());
        assert_eq!(info.common, None);
        assert_eq!(info.score_info.prop_score, 0);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, false, false, false));
    }
}
