use crate::characters::text::join;
use crate::conn::kani_context::KaniranContext;
use crate::dict::scoring::calc_score::calc_score;
use crate::dict::text_classes::{CompoundText, ScoreMod};
use crate::dict::accessors::get_kana;
use crate::dict::accessors::get_text;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::scoring::score::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment};
use crate::dict::accessors::set_word_conjugations;
use crate::dict::dao::WordConjugations;
use crate::dict::split::kani_hint_kind::KaniHintKind;
use crate::dict::split::kani_split_engine::{
    Finder, Len, Modify, PartSeq, Pred, SplitDef, Step, WordPart,
};
use crate::dict::split::kani_split_part::SplitPart;
use crate::dict::split::split::get_split;
use crate::dict::accessors::word_conj_data;
use std::sync::OnceLock;

/// Port of `ichiran/dict:*segsplit-map*` (`dict-split.lisp:704`).
///
/// seq → segment-split definition, for splits applied during
/// segmentation rather than word lookup.
/// One registered segsplit callsite. Wraps [`SplitDef`] (run by
/// [`crate::dict::split::kani_split_engine::run_split`] via
/// [`crate::dict::split::split_map::split_map_dispatch`] when
/// [`crate::dict::split::split_map::SplitMapKind::SegSplit`] is
/// active) with the keyword attrs destructured at
/// `dict-split.lisp:790`. Defaults reproduce the `&key` defaults of
/// that destructure for forms whose score-list omits a keyword.
pub struct SegSplitDef {
    pub split: SplitDef,
    pub primary: usize,
    pub connector: &'static str,
    pub root: &'static [usize],
}

pub static SEGSPLIT_TABLE: &[SegSplitDef] = &[
    // dict-split.lisp:761 (def-simple-split split-dakara) — score '(-5)
    SegSplitDef {
        split: SplitDef {
            seq: 1007310,
            score: -5,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2089020i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1002980i32]),
                    length: Len::Open,
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:744 (def-simple-split split-deha) — score '(-5)
    SegSplitDef {
        split: SplitDef {
            seq: 1008450,
            score: -5,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028980i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028920i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:707 (def-simple-split split-tokoroga) — score '(-10)
    SegSplitDef {
        split: SplitDef {
            seq: 1008570,
            score: -10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1343100i32]),
                    length: Len::LenMinus(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028930i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:778 (def-simple-split nil 1010105) — score '(5)
    SegSplitDef {
        split: SplitDef {
            seq: 1010105,
            score: 5,
            steps: &[
                Step::Test {
                    pred: Pred::TextEquals("はぐったり"),
                    score_mod: None,
                    push: None,
                },
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028920i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1004070i32]),
                    length: Len::Open,
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:752 (def-simple-split split-honno) — score '(-5)
    SegSplitDef {
        split: SplitDef {
            seq: 1011740,
            score: -5,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1522150i32]),
                    length: Len::LenMinus(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1469800i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:756 (def-simple-split split-kanatte) — score '(5)
    SegSplitDef {
        split: SplitDef {
            seq: 1208870,
            score: 5,
            steps: &[
                Step::Test {
                    pred: Pred::TextEquals("かなって"),
                    score_mod: None,
                    push: None,
                },
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1002940i32]),
                    length: Len::Fixed(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2086960i32]),
                    length: Len::Fixed(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:711 (def-simple-split split-tokorode) — score '(-10 :root (1))
    SegSplitDef {
        split: SplitDef {
            seq: 1343110,
            score: -10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1343100i32]),
                    length: Len::LenMinus(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028980i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[1],
    },
    // dict-split.lisp:736 (def-simple-split split-hitorashii) — score '(-10 :connector "")
    SegSplitDef {
        split: SplitDef {
            seq: 1366490,
            score: -10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1580640i32]),
                    length: Len::LenMinus(3),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1013240i32]),
                    length: Len::Open,
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: "",
        root: &[],
    },
    // dict-split.lisp:773 (def-simple-split nil 1567610) — score '(5)
    SegSplitDef {
        split: SplitDef {
            seq: 1567610,
            score: 5,
            steps: &[
                Step::Test {
                    pred: Pred::TextEquals("もんだ"),
                    score_mod: None,
                    push: None,
                },
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1502390i32]),
                    length: Len::Fixed(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2089020i32]),
                    length: Len::Open,
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:765 (def-simple-split nil 1675330) — score '(10 :primary 1)
    SegSplitDef {
        split: SplitDef {
            seq: 1675330,
            score: 10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1002980i32]),
                    length: Len::Fixed(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1260720i32]),
                    length: Len::Open,
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 1,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:727 (def-simple-split split-tokorodewa) — score '(-10)
    SegSplitDef {
        split: SplitDef {
            seq: 1897510,
            score: -10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1343100i32]),
                    length: Len::LenMinus(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028980i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028920i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:715 (def-simple-split split-dokoroka) — score '(-10)
    SegSplitDef {
        split: SplitDef {
            seq: 2009220,
            score: -10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1343100i32]),
                    length: Len::LenMinus(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028970i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:740 (def-simple-split split-toha) — score '(-5)
    SegSplitDef {
        split: SplitDef {
            seq: 2028950,
            score: -5,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1008490i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2028920i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:719 (def-simple-split split-tokoroe) — score '(-10)
    SegSplitDef {
        split: SplitDef {
            seq: 2097010,
            score: -10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1343100i32]),
                    length: Len::LenMinus(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2029000i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:723 (def-simple-split split-tokorowo) — score '(-10)
    SegSplitDef {
        split: SplitDef {
            seq: 2136660,
            score: -10,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1343100i32]),
                    length: Len::LenMinus(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2029010i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:748 (def-simple-split split-naito) — score '(-5)
    SegSplitDef {
        split: SplitDef {
            seq: 2394710,
            score: -5,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1529520i32]),
                    length: Len::Fixed(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1008490i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
    // dict-split.lisp:732 (def-simple-split split-omise) — score '(20 :primary 1 :connector "")
    SegSplitDef {
        split: SplitDef {
            seq: 2409240,
            score: 20,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2826528i32]),
                    length: Len::Fixed(1),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1582120i32]),
                    length: Len::Open,
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 1,
        connector: "",
        root: &[],
    },
    // dict-split.lisp:769 (def-simple-split nil 2841254) — score '(5)
    SegSplitDef {
        split: SplitDef {
            seq: 2841254,
            score: 5,
            steps: &[
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[1002980i32]),
                    length: Len::Fixed(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
                Step::Word(WordPart {
                    seq: PartSeq::Static(&[2086960i32]),
                    length: Len::Fixed(2),
                    finder: Finder::Seq,
                    modify: Modify::None,
                }),
            ],
        },
        primary: 0,
        connector: " ",
        root: &[],
    },
];

#[cfg(test)]
pub(crate) const REGISTERED_COUNT: usize = SEGSPLIT_TABLE.len();

/// Port of `ichiran/dict:*kana-hint-mod*` (`dict-split.lisp:813`).
///
/// Sentinel character marking a kana-particle boundary that the
/// romanizer should rewrite (`は → wa`, `へ → e`, …).
pub const KANA_HINT_MOD: char = '\u{200c}';

/// Port of `ichiran/dict:*kana-hint-space*` (`dict-split.lisp:814`).
///
/// Sentinel character marking hint-injected spaces in kana strings,
/// distinguishing them from real spaces in the source text.
pub const KANA_HINT_SPACE: char = '\u{200b}';

/// Port of `ichiran/dict:*hint-char-map*` (`dict-split.lisp:816`).
///
/// Maps each [`crate::dict::split::kani_hint_kind::KaniHintKind`] tag to the
/// sentinel character the hint system splices into a kana string at
/// that tag's position.
pub const HINT_CHAR_MAP: [(KaniHintKind, char); 2] = [
    (KaniHintKind::Space, KANA_HINT_SPACE),
    (KaniHintKind::Mod, KANA_HINT_MOD),
];

/// Port of `ichiran/dict:*hint-simplify-map*` (`dict-split.lisp:818-824`).
///
/// Ordered (from, to) substitution table that folds the hint
/// sentinels back into reader-facing characters:
///
/// - `*kana-hint-space*` → ASCII space `" "`
/// - `*kana-hint-mod*` + `は` → `わ`  (and `ハ` → `ワ`)
/// - `*kana-hint-mod*` + `へ` → `え`  (and `ヘ` → `エ`)
/// - lone `*kana-hint-mod*` → empty string (drop)
///
/// Order matters: the 2-char sentinel+kana entries must precede the
/// lone-sentinel entry so the longer match wins at the same offset.
pub fn hint_simplify_map() -> &'static [(String, &'static str)] {
    static CACHE: OnceLock<Vec<(String, &'static str)>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            vec![
                (KANA_HINT_SPACE.to_string(), " "),
                ([KANA_HINT_MOD, 'は'].iter().collect(), "わ"),
                ([KANA_HINT_MOD, 'ハ'].iter().collect(), "ワ"),
                ([KANA_HINT_MOD, 'へ'].iter().collect(), "え"),
                ([KANA_HINT_MOD, 'ヘ'].iter().collect(), "エ"),
                (KANA_HINT_MOD.to_string(), ""),
            ]
        })
        .as_slice()
}

/// Port of `ichiran/dict:get-segsplit` (`dict-split.lisp:784`).
///
/// When `segment.word` is a `simple-text`, dispatch through
/// `*segsplit-map*` for a split that decomposes the reading; on a hit,
/// wrap the parts in a `compound-text`, copy the segment, and overwrite
/// word/text/score/info on the copy. Returns `None` for
/// non-`simple-text` words and for readings with no matching segsplit
/// entry.
pub fn get_segsplit(
    ctx: &KaniranContext,
    segment: &Segment,
) -> Result<Option<Segment>, crate::conn::KaniDbError> {
    // dict-split.lisp:785 (when (typep word 'simple-text))
    let simple_word = match &segment.word {
        KaniWordDispatchEnum::Kanji(k) => KaniSimpleTextDispatchEnum::Kanji(k.clone()),
        KaniWordDispatchEnum::Kana(k) => KaniSimpleTextDispatchEnum::Kana(k.clone()),
        KaniWordDispatchEnum::Proxy(p) => KaniSimpleTextDispatchEnum::Proxy(p.clone()),
        KaniWordDispatchEnum::Compound(_) | KaniWordDispatchEnum::Counter(_) => {
            return Ok(None);
        }
    };

    // dict-split.lisp:786 (let ((*split-map* *segsplit-map*)) …)
    let ctx2 = ctx.with_segsplit_map();

    // dict-split.lisp:788 (cdr (getf (segment-info segment) :seq-set))
    let conj_of: Vec<i32> = segment
        .info
        .as_ref()
        .map(|info| info.seq_set.iter().skip(1).copied().collect())
        .unwrap_or_default();

    // dict-split.lisp:787-788
    // (multiple-value-bind (split attrs) (get-split word conj-of) …)
    let Some((parts, score)) = get_split(&ctx2, &simple_word, &conj_of)? else {
        return Ok(None);
    };

    // dict-split.lisp:790 (destructuring-bind (score &key (primary 0) (connector " ") root) attrs)
    // attrs == the score-arg list passed to `def-simple-split` (e.g.
    // '(-10 :root (1))). The Rust SEGSPLIT_TABLE stores the destructured
    // slots alongside SplitDef; recover them by re-walking the same seq
    // order get-split* used.
    let Some(def) = find_segsplit_def(simple_word.seq(), &conj_of) else {
        return Ok(None);
    };

    let mut words: Vec<KaniWordDispatchEnum> = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            SplitPart::Word(w) => words.push(w),
            // SEGSPLIT_TABLE has no Step::Push rows — the registered
            // segsplit forms at dict-split.lisp:706-782 never pass `:score`
            // / `:pscore` as parts. (setf word-conjugations) on a keyword
            // would signal no-applicable-method upstream.
            SplitPart::Score | SplitPart::PScore => unreachable!(
                "segsplit-map split returned :score / :pscore part — not in SEGSPLIT_TABLE shape"
            ),
        }
    }

    // dict-split.lisp:799-801 (when root (loop for i from 0 for word in split
    //                            if (find i root) do (setf (word-conjugations word) :root)))
    if !def.root.is_empty() {
        for (i, w) in words.iter_mut().enumerate() {
            if def.root.contains(&i) {
                set_word_conjugations(w, Some(WordConjugations::Root));
            }
        }
    }

    // dict-split.lisp:793 (:text (join "" (mapcar 'get-text split)))
    let texts: Vec<String> = words.iter().map(|w| get_text(w).into_owned()).collect();
    // dict-split.lisp:794 (:kana (join connector (mapcar 'get-kana split)))
    let mut kanas: Vec<String> = Vec::with_capacity(words.len());
    for w in &words {
        // dict.lisp:638 precedent — `(concatenate 'string nil ...)` treats
        // nil as empty; `Option<String>` from get_kana mirrors with
        // `.unwrap_or_default()`.
        kanas.push(get_kana(ctx, w)?.unwrap_or_default());
    }

    // dict-split.lisp:795 (:primary (elt split primary))
    let primary_word = Box::new(words[def.primary].clone());

    // dict-split.lisp:791-797 (make-instance 'compound-text …)
    let compound = CompoundText {
        text: join("", &texts),
        kana: join(def.connector, &kanas),
        primary: primary_word,
        words,
        // dict.lisp:613 — `score-base :initform nil :initarg :score-base`;
        // not passed by get-segsplit, slot keeps default nil.
        score_base: None,
        // dict-split.lisp:797 (:score-mod score) — `score` here is the
        // integer head of the attrs list (the destructured `score`
        // positional), held in SplitDef.score.
        score_mod: ScoreMod::Single(score as i64),
    };

    let wrapped = KaniWordDispatchEnum::Compound(compound);

    // dict-split.lisp:805 (nth-value 1 (calc-score (primary word)))
    let primary_ref: &KaniWordDispatchEnum = match &wrapped {
        KaniWordDispatchEnum::Compound(c) => &c.primary,
        _ => unreachable!(),
    };
    let (_disc_score, info_opt) = calc_score(ctx, primary_ref, false, None, None, &[])?;

    // dict-split.lisp:806 (getf (segment-info new-seg) :conj) (word-conj-data word)
    let conj_data = word_conj_data(ctx, &wrapped)?;
    // CL setf-getf-on-nil idiom: when `info` is nil, `(setf (getf nil :conj) X)`
    // rewrites the binding to a fresh plist `(:conj X)`. Mirror by
    // synthesizing the same zero/empty KaniSegmentInfo calc_score uses
    // for the analogous compound-recursion path (calc_score.rs:238-250).
    let mut info = info_opt.unwrap_or_else(|| KaniSegmentInfo {
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
    info.conj = conj_data;

    // dict-split.lisp:798 (new-seg (copy-segment segment))
    let mut new_seg = segment.clone();

    // dict-split.lisp:802-807 — parallel setf populates new-seg slots.
    // Compute reads before the moves so the source values come from a
    // consistent snapshot, matching the parallel-setf semantics.
    let new_text = get_text(&wrapped).into_owned();
    let old_score = segment
        .score
        .expect("get-segsplit: segment.score must be set by gen-score before call");

    new_seg.word = wrapped;
    new_seg.text = Some(new_text);
    new_seg.score = Some(old_score + score);
    new_seg.info = Some(info);

    Ok(Some(new_seg))
}

fn find_segsplit_def(seq: i32, conj_of: &[i32]) -> Option<&'static SegSplitDef> {
    // dict-split.lisp:70 — first lookup is (gethash (seq reading) *split-map*).
    if let Some(def) = SEGSPLIT_TABLE.iter().find(|d| d.split.seq == seq) {
        return Some(def);
    }
    // dict-split.lisp:73-75 — then walk conj-of in order, returning the
    // first match.
    for &s in conj_of {
        if let Some(def) = SEGSPLIT_TABLE.iter().find(|d| d.split.seq == s) {
            return Some(def);
        }
    }
    None
}

#[cfg(test)]
mod tests;
