use crate::conn::kani_context::KaniranContext;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;
use crate::dict::split::kani_split_engine::{
    run_split, Finder, Len, Modify, PartSeq, Pred, ScorePush, SplitDef, Step, WordPart,
};
use crate::dict::split::kani_split_part::SplitPart;
use crate::dict::split::segsplit::SEGSPLIT_TABLE;
use crate::dict::word_type::WordType;

/// Port of `ichiran/dict:*split-map*` (`dict-split.lisp:5`).
///
/// JMdict seq → split definition, used to split a single dictionary
/// entry into its component words.
/// Selector for the active `*split-map*` binding. Diverges from
/// upstream "any hashtable" value space — closed to the two tables
/// upstream actually binds (`*split-map*` itself or `*segsplit-map*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitMapKind {
    /// `*split-map*` (`dict-split.lisp:5`) — [`SPLIT_TABLE`].
    Default,
    /// `*segsplit-map*` (`dict-split.lisp:704`) —
    /// [`crate::dict::split::segsplit::SEGSPLIT_TABLE`].
    SegSplit,
}

fn split_2529050_first_part_len(txt: &str, _len_: usize) -> Option<usize> {
    Some(if txt.starts_with("もの") { 2 } else { 1 })
}

fn split_hayaimonode_second_part_len(txt: &str, _len_: usize) -> Option<usize> {
    Some(if txt.contains('物') { 1 } else { 2 })
}

fn split_hitotachi_first_part_len(txt: &str, _len_: usize) -> Option<usize> {
    Some(if txt.chars().any(|c| c == '人') {
        1
    } else {
        2
    })
}

fn split_hitotachi_second_part_len(txt: &str, _len_: usize) -> Option<usize> {
    Some(if txt.chars().any(|c| c == '達') {
        1
    } else {
        2
    })
}

pub static SPLIT_TABLE: &[SplitDef] = &[
    SplitDef {
        seq: 1000430,
        score: -5,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Static(&[1000420i32]),
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 1002970,
        score: 600,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2143350i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "知れない",
                    seq: 1420490,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1004800,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1628530i32]),
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
    SplitDef {
        seq: 1005600,
        score: -10,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Dynamic {
                text: "しまった",
                seq: 1305380,
            },
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 1005700,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1156990i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1005830,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1370760i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1006280,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1157170i32]),
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
    SplitDef {
        seq: 1006650,
        score: 5,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2137720i32]),
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
    SplitDef {
        seq: 1006840,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1006880i32]),
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
    SplitDef {
        seq: 1006880,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1006830i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1352130i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1008030,
        score: -10,
        steps: &[Step::Push(ScorePush::Score)],
    },
    SplitDef {
        seq: 1009470,
        score: 1,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Dynamic {
                text: "なら",
                seq: 2089020,
            },
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 1009600,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "取って",
                    seq: 1326980,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1157200,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2772730i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1157220,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1195970i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1157230,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1284430i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1157240,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1600260i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1157280,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1370090i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1157310,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1405800i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1163700,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1576150i32]),
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
    SplitDef {
        seq: 1164910,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2821500i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432920i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1189420,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2416780i32]),
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
    SplitDef {
        seq: 1207840,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "割り",
                    seq: 1208000,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1384860i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1221530,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1221520i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028930i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1296400i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1221680,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1221520i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1157170i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1221750,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1221520i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1469800i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1610040i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1236680,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1236660i32]),
                length: Len::CharPosPlus1('れ'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1465580i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1245390,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1245290i32]),
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
    SplitDef {
        seq: 1260990,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1260670i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1270210,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1001640i32]),
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
    SplitDef {
        seq: 1272220,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1592990i32]),
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
    SplitDef {
        seq: 1304820,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1207610i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1304890,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1256520i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1304960,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1307550i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1305110,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1338180i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1305280,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1599390i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1305290,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1212670i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1311360,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1311350i32]),
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
    SplitDef {
        seq: 1314600,
        score: -5,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1314770,
        score: -10,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1495740i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1315700,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "持って",
                    seq: 1315720,
                },
                length: Len::CharPosPlus1('て'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1578850i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1315860,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1315840i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2215430i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1322540,
        score: -5,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1322560,
        score: -10,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1226480i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1327220,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1327190i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1465590i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1327230,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1327190i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1465610i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1343110,
        score: 20,
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
    SplitDef {
        seq: 1349300,
        score: 5,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2029110i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2826528i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1362970,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "申し",
                    seq: 1363090,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1589040i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1368500,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1368490i32]),
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
    SplitDef {
        seq: 1368740,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1580640i32]),
                length: Len::Compute(split_hitotachi_first_part_len),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1416220i32]),
                length: Len::Compute(split_hitotachi_second_part_len),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1368820,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1580640i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1395670,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1395660i32]),
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
    SplitDef {
        seq: 1411570,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1590770i32, 1510720i32]),
                length: Len::CharPosPlus1('り'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "映え",
                    seq: 1600620,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1414570,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2082450i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1417790,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1417780i32]),
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
    SplitDef {
        seq: 1419350,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1901710i32]),
                length: Len::CharPosPlus1('き'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1338180i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1424950,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1620400i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1424960,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1423310i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1454270,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1454260i32]),
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
    SplitDef {
        seq: 1462720,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1461140i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432920i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1465460,
        score: 100,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "入り",
                    seq: 1465590,
                },
                length: Len::CharPosPlus1('り'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1288790i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1474200,
        score: -10,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
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
                seq: PartSeq::Static(&[1577980i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1479100,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1679020i32]),
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
    SplitDef {
        seq: 1489800,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1489340i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1502500,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1502390i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1277450i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::Unrendaku,
            }),
        ],
    },
    SplitDef {
        seq: 1508380,
        score: 10,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2083990i32]),
                length: Len::Fixed(3),
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
    SplitDef {
        seq: 1510140,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1680900i32]),
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
    SplitDef {
        seq: 1518540,
        score: 10,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "亡く",
                    seq: 1518450,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1375610i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1518550,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529560i32]),
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
    SplitDef {
        seq: 1523010,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1522150i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1524660,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1524640i32]),
                length: Len::CharPos('に'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1529550,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "無く",
                    seq: 1529520,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1375610i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1530610,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1530600i32]),
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
    SplitDef {
        seq: 1531420,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1531410i32]),
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
    SplitDef {
        seq: 1532270,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "あけまして",
                    seq: 1202450,
                },
                length: Len::Fixed(5),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1001540i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1538340,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1538330i32]),
                length: Len::CharPos('が'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028930i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1606560i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1550490,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1550190i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1551500,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "立ち",
                    seq: 1597040,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1570220i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1579130,
        score: -1,
        steps: &[
            Step::Test {
                pred: Pred::TextEquals("ことし"),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1313580i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2086640i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1581550,
        score: 10,
        steps: &[
            Step::Test {
                pred: Pred::TextStartsWith("雪"),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1386500i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028930i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Test {
                pred: Pred::LenGt(2),
                score_mod: Some(-2),
                push: Some(ScorePush::PScore),
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1591050,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1221520i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028930i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1495740i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1591980,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1221520i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2029010i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1305990i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1594300,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1596510i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1594310,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1406680i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1594460,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1372620i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1594580,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1277100i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1597400,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1585205i32]),
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
    SplitDef {
        seq: 1597740,
        score: 5,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1008030i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2081610i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1599590,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1188490i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2143350i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1601010,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "跳ね",
                    seq: 1429620,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1352290i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1601080,
        score: -5,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028920i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1310680i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1602740,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1497180i32]),
                length: Len::LenMinus(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2093780i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1606530,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "分かり",
                    seq: 1606560,
                },
                length: Len::Fixed(3),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1384830i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1606800,
        score: 10,
        steps: &[
            Step::Test {
                pred: Pred::LenEq(2),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "割り",
                    seq: 1208000,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1609470,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1514990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1005340i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1611020,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1577100i32]),
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
    SplitDef {
        seq: 1612640,
        score: 5,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1000420i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2029080i32, 2029120i32, 1005110i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1619440,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2069220i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1679990,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2582460i32]),
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
    SplitDef {
        seq: 1682060,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2085340i32]),
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
    SplitDef {
        seq: 1693800,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2826528i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1609810i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1736650,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1611710i32]),
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
    SplitDef {
        seq: 1752860,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1636070i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "支え",
                    seq: 1310090,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1774820,
        score: -5,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1002980i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1277450i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1808080,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1604890i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1820790,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kanji),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1250090i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1432930i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1854750,
        score: 20,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Dynamic {
                text: "付いて",
                seq: 1495740,
            },
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 1855670,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "取り留め",
                    seq: 1707770,
                },
                length: Len::CharPos('の'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1469800i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1863230,
        score: 15,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1576870i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1416220i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1865020,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1590150i32]),
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
    SplitDef {
        seq: 1878880,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2423450i32]),
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
    SplitDef {
        seq: 1881080,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1310720i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1207590i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1881690,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1321900i32]),
                length: Len::CharPos('を'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2029010i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1298790i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1894260,
        score: 50,
        steps: &[
            Step::Test {
                pred: Pred::LenGt(3),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "付いて",
                    seq: 1894260,
                },
                length: Len::Fixed(3),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1577980i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::OptPrefix("い"),
            }),
        ],
    },
    SplitDef {
        seq: 1903910,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1601890i32]),
                length: Len::CharPos('に'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "なって",
                    seq: 1375610,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1922760,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1008490i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1587040i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 1951150,
        score: 50,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Dynamic {
                text: "決まって",
                seq: 1591420,
            },
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 2002270,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "零れ",
                    seq: 1557650,
                },
                length: Len::CharPosPlus1('れ'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1548550i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2007500,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "落ち",
                    seq: 1548550,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1557650i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2009290,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1423310i32]),
                length: Len::LenMinus(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1008460i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2016840,
        score: -5,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Dynamic {
                text: "やった",
                seq: 1012980,
            },
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 2026650,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "せよ",
                    seq: 1157170,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2034520,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028980i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2827091i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2057340,
        score: 300,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1363050i32]),
                length: Len::CharPos('な'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2246510i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2083990,
        score: 20,
        steps: &[
            Step::Test {
                pred: Pred::TextEquals("ならん"),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1009470i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2139720i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2088480,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1634130i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2006580i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2089710,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1327190i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028930i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1207590i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2100770,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1008490i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "なったら",
                    seq: 1375610,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2100900,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1008490i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1375610i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2102910,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1258950i32]),
                length: Len::CharPos('を'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2029010i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1508390i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2104540,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1188420i32]),
                length: Len::CharPosPlus1('か'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1375610i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2109610,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "有り",
                    seq: 1296400,
                },
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1588760i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2126220,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1802920i32]),
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
    SplitDef {
        seq: 2133750,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1224890i32]),
                length: Len::CharPosPlus1('く'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1001720i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2135280,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2089020i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2139720i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2136520,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2005870i32]),
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
    SplitDef {
        seq: 2142680,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2252690i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1290210i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2142710,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2252690i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1185200i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2215340,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1313580i32]),
                length: Len::CharPos('に'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1157170i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2253080,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028980i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1612690i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2272780,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1221520i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028930i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2276360,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2436480i32]),
                length: Len::LenMinus(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2086640i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2433760,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1006610i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2683060i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2513590,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2513650i32]),
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
    SplitDef {
        seq: 2518250,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1332760i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2523480,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2252690i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1442750i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2526850,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028990i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "しろ",
                    seq: 1157170,
                },
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2529050,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1322990i32]),
                length: Len::Compute(split_2529050_first_part_len),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1234250i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2610760,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "弾け",
                    seq: 1419380,
                },
                length: Len::CharPosPlus1('け'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1429700i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2612990,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1922760i32]),
                length: Len::Fixed(3),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1313580i32]),
                length: Len::LenMinus(4),
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
    SplitDef {
        seq: 2666360,
        score: 30,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Dynamic {
                text: "少なくない",
                seq: 1348910,
            },
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 2668400,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1213060i32]),
                length: Len::CharPos('を'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2029010i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1552120i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2719270,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1445430i32]),
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
    SplitDef {
        seq: 2724560,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1469800i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1610040i32]),
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
        ],
    },
    SplitDef {
        seq: 2755350,
        score: 10,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2089020i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2757500,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1538330i32]),
                length: Len::CharPos('の'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1469800i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1606560i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2757540,
        score: 90,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1896380i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2728200i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2762260,
        score: 0,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Dynamic {
                text: "ならんで",
                seq: 1508380,
            },
            length: Len::Open,
            finder: Finder::Seq,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 2771850,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2563780i32]),
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
    SplitDef {
        seq: 2771940,
        score: -5,
        steps: &[
            Step::Test {
                pred: Pred::TextEquals("はないか"),
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
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Fixed(2),
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
    SplitDef {
        seq: 2800540,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2252690i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028930i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1495740i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2803190,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2252690i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1595630i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2810720,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1004820i32]),
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
    SplitDef {
        seq: 2810800,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1587590i32]),
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
    SplitDef {
        seq: 2815260,
        score: 100,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1404975i32]),
                length: Len::CharPosPlus1('い'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1502390i32]),
                length: Len::Compute(split_hayaimonode_second_part_len),
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
    SplitDef {
        seq: 2819990,
        score: 20,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "じゃない",
                    seq: 2089020,
                },
                length: Len::Fixed(4),
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
    SplitDef {
        seq: 2834051,
        score: 15,
        steps: &[
            Step::Test {
                pred: Pred::WordType(WordType::Kana),
                score_mod: None,
                push: None,
            },
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1002290i32]),
                length: Len::Fixed(3),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1416220i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2834732,
        score: -10,
        steps: &[Step::Word(WordPart {
            seq: PartSeq::Static(&[1707770i32]),
            length: Len::Open,
            finder: Finder::ConjOf,
            modify: Modify::None,
        })],
    },
    SplitDef {
        seq: 2835890,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1385860i32]),
                length: Len::Fixed(5),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1319060i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2837492,
        score: 5,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2137720i32]),
                length: Len::Fixed(2),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1628500i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2846470,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1221520i32]),
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
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2855921,
        score: 50,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "取り留め",
                    seq: 1707770,
                },
                length: Len::CharPos('も'),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[2028940i32]),
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1529520i32]),
                length: Len::Open,
                finder: Finder::Seq,
                modify: Modify::None,
            }),
        ],
    },
    SplitDef {
        seq: 2858937,
        score: 30,
        steps: &[
            Step::Word(WordPart {
                seq: PartSeq::Dynamic {
                    text: "し",
                    seq: 1157170,
                },
                length: Len::Fixed(1),
                finder: Finder::Seq,
                modify: Modify::None,
            }),
            Step::Word(WordPart {
                seq: PartSeq::Static(&[1406690i32]),
                length: Len::Open,
                finder: Finder::ConjOf,
                modify: Modify::None,
            }),
        ],
    },
];

pub async fn split_map_dispatch(
    seq: i32,
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
) -> Option<Result<(Vec<Option<SplitPart>>, i32), sqlx::Error>> {
    match ctx.split_map {
        SplitMapKind::Default => {
            let def = SPLIT_TABLE.iter().find(|d| d.seq == seq)?;
            Some(run_split(def, ctx, reading).await)
        }
        SplitMapKind::SegSplit => {
            let def = SEGSPLIT_TABLE.iter().find(|d| d.split.seq == seq)?;
            Some(run_split(&def.split, ctx, reading).await)
        }
    }
}

/// Number of registered seqs — pinned so the build fails loudly if
/// a future macro form accidentally drops out of the regenerated set.
#[cfg(test)]
pub(crate) const REGISTERED_COUNT: usize = SPLIT_TABLE.len();

#[cfg(test)]
mod tests;
