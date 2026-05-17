//! Port of `ichiran/dict:*segsplit-map*` (`dict-split.lisp:704`).
//!
//! 18 `def-simple-split` callsites registered inside
//! `(let ((*split-map* *segsplit-map*)) ...)` at
//! `dict-split.lisp:706-782` — the let-binding redirects `defsplit`'s
//! `(setf (gethash ,seq *split-map*) ...)` target. Collapsed to a
//! static [`SEGSPLIT_TABLE`] of data rows, mirroring
//! [`super::_star_split_map_star_`]; both tables are dispatched via
//! [`super::_star_split_map_star_::split_map_dispatch`] keyed off
//! [`crate::conn::kani_context::KaniranContext::split_map`].
//!
//! Diverges from `*split-map*`: each callsite here passes a **list**
//! as the macro's `score` arg (e.g. `'(-10 :root (1))` at
//! `dict-split.lisp:711`, `'(20 :primary 1 :connector "")` at `:732`).
//! Upstream `get-segsplit` destructures it as
//! `(score &key (primary 0) (connector " ") root)` at
//! `dict-split.lisp:790`. The port stores the destructured slots
//! alongside the [`SplitDef`] on [`SegSplitDef`]; the integer score
//! lives on `SplitDef.score`. `get-segsplit` recovers the keyword
//! attrs via a direct [`SEGSPLIT_TABLE`] walk.
//!
//! Diverges from CONVENTIONS §1: the 18 `split-*` callsites collapse
//! to data rows here rather than per-file ports, same rationale as
//! [`super::_star_split_map_star_`].

use crate::dict::kani_split_engine::{
    Finder, Len, Modify, PartSeq, Pred, SplitDef, Step, WordPart,
};

/// One registered segsplit callsite. Wraps [`SplitDef`] (run by
/// [`super::kani_split_engine::run_split`] via
/// [`super::_star_split_map_star_::split_map_dispatch`] when
/// [`crate::dict::_star_split_map_star_::SplitMapKind::SegSplit`] is
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
