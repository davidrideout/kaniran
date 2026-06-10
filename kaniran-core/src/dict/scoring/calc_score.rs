use crate::characters::char_class::{count_char_class, CharClass};
use crate::characters::kana::mora_length;
use crate::conn::kani_context::KaniranContext;
use crate::dict::accessors::{
    apply_score_mod, get_original_text, score_base, true_text, word_conj_data, word_conjugations,
    word_type, WordType,
};
use crate::dict::conj::ConjData;
use crate::dict::counters::classes::Common;
use crate::dict::counters::methods::{
    common as common_fn, nokanji, ord as ord_fn, text as text_fn,
};
use crate::dict::dao::{Entry, SimpleText, WordConjugations};
use crate::dict::errata::{
    semi_final_prt, skip_by_conj_data, test_conj_prop, COPULAE, FINAL_PRT, NON_FINAL_PRT,
    SKIP_WORDS, WEAK_CONJ_FORMS,
};
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::scoring::score::{
    compare_common, get_non_arch_posi, is_arch as is_arch_fn, kanji_break_penalty,
    length_multiplier_coeff, KaniLengthClass, KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo,
};
use crate::dict::split::kani_split_part::SplitPart;
use crate::dict::split::split::get_split;
use crate::dict::text_classes::{ProxyText, ScoreMod};
use std::borrow::Cow;

/// Port of `ichiran/dict:calc-score` (`dict.lisp:775`).
///
/// Computes the segmenter's word-candidate score and the
/// [`KaniSegmentInfo`] property bag that the downstream scoring loop
/// (penalties, synergies, kanji-break) reads, dispatching on
/// compound-text, counter-text, and simple-text readings.
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
            ctx,
            base,
            false,
            use_length_rec,
            score_mod_rec,
            &[],
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
    let katakana_p = !kanji_p && count_char_class(&true_text(reading), CharClass::KatakanaUniq) > 0;
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
            Some(crate::dict::counters::classes::CounterSource::Kanji(k)) => Some(k.seq),
            Some(crate::dict::counters::classes::CounterSource::Kana(k)) => Some(k.seq),
            None => None,
        },
        KaniWordDispatchEnum::Compound(_) => {
            unreachable!("compound-text returned early on the typecase branch")
        }
    };
    let mut ord: i32 = ord_fn(reading);

    // dict.lisp:803 (entry (and seq (get-dao 'entry seq)))
    let entry: Option<Entry> = match seq {
        Some(s) => ctx.store.entry_by_seq(s).await?,
        None => None,
    };

    // dict.lisp:804 (conj-only (let ((wc (word-conjugations reading))) (and wc (not (eql wc :root)))))
    let wc = word_conjugations(reading);
    let conj_only = matches!(wc, Some(WordConjugations::Ids(_)));

    // dict.lisp:805 (root-p (or ctr-mode (and (not conj-only) (root-p entry))))
    let root_p = ctr_mode || (!conj_only && entry.as_ref().map(|e| e.root_p).unwrap_or(false));

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
    let conj_props: Vec<&crate::dict::dao::ConjProp> = conj_data
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
        ctx.store.uk_sense_ids(&sp_seq_set).await?
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
    let semi_final_particle_p = seq.is_some_and(|s| semi_final_prt().contains(&s));
    let non_final_particle_p = seq.is_some_and(|s| NON_FINAL_PRT.contains(&s));
    let pronoun_p = posi.iter().any(|s| s == "pn");
    let cop_da_p = seq_set.iter().any(|s| COPULAE.contains(s));

    // dict.lisp:836-844 (long-p …)
    let long_threshold: usize = if kanji_p
        && !prefer_kana
        && ((root_p && conj_data.is_empty()) || (use_length.is_some() && conj_types.contains(&13)))
    {
        2
    } else if common_p && common_value > 0 && common_value < 10 {
        2
    } else if (conj_types.contains(&3) || conj_types.contains(&9)) && use_length.is_none() {
        4
    } else {
        3
    };
    let long_p = len > long_threshold;

    // dict.lisp:845-847 (no-common-bonus …)
    let no_common_bonus =
        particle_p || !conj_types_p || (!long_p && posi.len() == 1 && posi[0] == "int");

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
        let orig_texts = get_original_text(ctx, &reading_simple, Some(&conj_data)).await?;

        // dict.lisp:860-861 collect (list (common ot) (ord ot))
        let conj_of_data: Vec<(Option<i32>, i32)> = orig_texts
            .iter()
            .map(|ot| match ot {
                KaniSimpleTextDispatchEnum::Kanji(k) => (k.common, k.ord),
                KaniSimpleTextDispatchEnum::Kana(k) => (k.common, k.ord),
                KaniSimpleTextDispatchEnum::Proxy(_) => {
                    unreachable!("get-original-text returns kanji/kana variants only")
                }
            })
            .collect();

        if !conj_of_data.is_empty() {
            // dict.lisp:863-867 (unless common-p …) — only collect/sort conj-of-common
            // when the candidate doesn't already have a common rank.
            if !common_p {
                let conj_of_common: Vec<i32> =
                    conj_of_data.iter().filter_map(|(c, _)| *c).collect();
                if !conj_of_common.is_empty() {
                    // dict.lisp:867 (car (sort conj-of-common #'compare-common))
                    // `compare-common` is a binary predicate (not a strict-weak
                    // ordering), so it's not safe to drive Rust's `sort_by` with
                    // it — only the single-minimum lookup the upstream `(car …)`
                    // expresses is well-defined. `min_by` is enough.
                    common_of = conj_of_common.iter().copied().min_by(|a, b| {
                        if compare_common(Some(*a as i64), Some(*b as i64)).is_truthy() {
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
                    && ((kanji_p && !prefer_kana) || (common_p && pronoun_p) || e.n_kanji == 0);
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
                    let any_sense_ord_zero: Option<i32> =
                        ctx.store.sense_id_ord0(&prefer_kana_sense_ids).await?;
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
            if kanji_p && primary_p {
                4
            } else {
                2
            }
        } else if long_p || cop_da_p || (root_p && (kanji_p || (primary_p && len > 2))) {
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
                        let truncated_text: String = ptext.chars().take(new_len).collect();
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
mod tests;
