//! Port of `ichiran/dict:*segsplit-map*` (`dict-split.lisp:704`).
//!
//! Hashtable mapping JMdict seq → segment-split function, registered
//! upstream by 18 `def-simple-split` callsites inside the
//! `(let ((*split-map* *segsplit-map*)) ...)` block at
//! `dict-split.lisp:706-782`. The let-binding redirects the
//! `defsplit` macro's `setf (gethash ,seq *split-map*) ',name` target
//! from `*split-map*` to `*segsplit-map*` at expansion-eval time, so
//! the same macro that populates `*split-map*` (174 callsites) lands
//! these 18 callsites here instead.
//!
//! The Rust transliteration collapses the runtime hashtable into a
//! static [`SEGSPLIT_TABLE`] of data rows, mirroring the convention
//! established by [`super::_star_split_map_star_`]. Each row is
//! interpreted by [`super::kani_split_engine::run_split`].
//!
//! ## Divergence from `*split-map*`: the score-var carries keyword attrs
//!
//! The 18 `def-simple-split` callsites here pass a **list** as the
//! `score` argument — e.g. `'(-10 :root (1))` (`dict-split.lisp:711`),
//! `'(20 :primary 1 :connector "")` (`:732`), `'(-10 :connector "")`
//! (`:736`), `'(10 :primary 1)` (`:765`). The macro binds this list
//! verbatim to the `prog*`'s `score-var` and returns it unchanged via
//! `(values (nreverse parts) score-var)` (`dict-split.lisp:67`). The
//! consumer [`get-segsplit`](https://github.com/tshatrov/ichiran)
//! (`dict-split.lisp:784`, FQN `ichiran/dict:get-segsplit`, port wave
//! 396) then `destructuring-bind`s `(score &key (primary 0)
//! (connector " ") root)` out of it.
//!
//! In Rust, [`SplitDef::score`] is a plain `i32` and the engine
//! ([`run_split`](super::kani_split_engine::run_split)) does not carry
//! the keyword payload through its prog\*-equivalent loop (the
//! upstream prog\* doesn't either — it just passes the list through
//! inert). The four destructured slots are stored alongside the
//! `SplitDef` on [`SegSplitDef`] and re-attached by
//! [`segsplit_map_dispatch`] into [`SegSplitAttrs`] on return,
//! preserving the upstream "funcall yields `(values parts attrs)`"
//! shape (`dict-split.lisp:72`, `:80`) without widening
//! `SplitDef.score` to support segsplit-only forms.
//!
//! Diverges from CONVENTIONS §1 (one Lisp symbol per Rust file): the
//! 18 `split-*` callsites that register here (wave 177-194) are not
//! given their own files — they collapse to data rows in
//! [`SEGSPLIT_TABLE`], same rationale as [`super::_star_split_map_star_`]
//! cites for its 174 callsites. `audit-signatures` reports each
//! `split-*` FQN as `port file not found` — those entries are this
//! convention.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani_split_engine::{
    run_split, Finder, Len, Modify, PartSeq, Pred, SplitDef, Step, WordPart,
};
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

/// Mirror of `(destructuring-bind (score &key (primary 0) (connector " ") root) attrs ...)`
/// at `dict-split.lisp:790`. Defaults reproduce the `&key` defaults at
/// that callsite for forms whose score-list omits a keyword.
pub struct SegSplitAttrs {
    pub score: i32,
    pub primary: usize,
    pub connector: &'static str,
    pub root: &'static [usize],
}

/// One registered segsplit callsite. Wraps [`SplitDef`] (consumed by
/// [`run_split`]) with the keyword attrs that ride alongside the score
/// in the upstream score-form.
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

/// Mirror of `(funcall split-fn reading)` at `dict-split.lisp:72`/`:74`
/// with `*split-map*` let-bound to `*segsplit-map*` (the redirection
/// at `dict-split.lisp:786` inside `get-segsplit`). Returns `None` for
/// unregistered seqs to preserve the upstream `(gethash seq
/// *segsplit-map*)` semantics. The keyword attrs travel back to the
/// caller bundled as [`SegSplitAttrs`], reproducing the
/// `destructuring-bind (score &key primary connector root) attrs`
/// shape at `dict-split.lisp:790`.
pub async fn segsplit_map_dispatch(
    seq: i32,
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
) -> Option<Result<(Vec<Option<SplitPart>>, SegSplitAttrs), sqlx::Error>> {
    let def = SEGSPLIT_TABLE.iter().find(|d| d.split.seq == seq)?;
    Some(run_split(&def.split, ctx, reading).await.map(|(parts, score)| {
        (
            parts,
            SegSplitAttrs {
                score,
                primary: def.primary,
                connector: def.connector,
                root: def.root,
            },
        )
    }))
}

#[cfg(test)]
pub(crate) const REGISTERED_COUNT: usize = SEGSPLIT_TABLE.len();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_count_matches_upstream_segsplit_map() {
        // dict-split.lisp:706-782 registers 18 entries via the 18
        // def-simple-split forms inside the let-binding that redirects
        // *split-map* to *segsplit-map*.
        assert_eq!(REGISTERED_COUNT, 18);
    }
}
