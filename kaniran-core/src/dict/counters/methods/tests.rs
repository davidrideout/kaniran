use super::*;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::counters::classes::{
    Common, Counter, CounterDaysOn, CounterHalfhour, CounterMonths, CounterSource, CounterText,
    CounterTsu, CounterWari,
};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::simple_text_class::SimpleText;

// --- common ---
fn kanji_with_common(c: Option<i32>) -> KanjiText {
    KanjiText {
        id: 0,
        seq: 0,
        text: String::new(),
        ord: 0,
        common: c,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji: false,
        best_kana: None,
        state: SimpleText::default(),
    }
}

fn kana_with_common(c: Option<i32>) -> KanaText {
    KanaText {
        id: 0,
        seq: 0,
        text: String::new(),
        ord: 0,
        common: c,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji: false,
        best_kanji: None,
        state: SimpleText::default(),
    }
}

fn counter(common_slot: Common, source: Option<CounterSource>) -> Counter {
    Counter::Base(CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: "0".into(),
        number: 0,
        source,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: common_slot,
        allowed: Vec::new(),
        foreign: false,
    })
}

#[test]
fn simple_text_some_score() {
    assert_eq!(
        common(&KaniWordDispatchEnum::Kanji(kanji_with_common(Some(5)))),
        Common::Score(5),
    );
}

#[test]
fn simple_text_none_is_null() {
    // `db-null` upstream → :null → Common::Null in Rust.
    assert_eq!(
        common(&KaniWordDispatchEnum::Kana(kana_with_common(None))),
        Common::Null,
    );
}

#[test]
fn counter_score_short_circuits() {
    let c = counter(
        Common::Score(7),
        Some(CounterSource::Kana(kana_with_common(Some(99)))),
    );
    assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(7));
}

#[test]
fn counter_explicit_null_short_circuits() {
    // Lisp `(or :null ...)` returns :null — :null is truthy so
    // the `or` does NOT recurse on source.
    let c = counter(
        Common::Null,
        Some(CounterSource::Kana(kana_with_common(Some(3)))),
    );
    assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Null);
}

#[test]
fn counter_inherit_recurses_on_source() {
    let c = counter(
        Common::Inherit,
        Some(CounterSource::Kanji(kanji_with_common(Some(11)))),
    );
    assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(11));
}

#[test]
fn counter_inherit_no_source_returns_zero() {
    // dict-counters.lisp:75-76 — `(or nil (if nil ... 0))` → 0.
    let c = counter(Common::Inherit, None);
    assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(0));
}

#[test]
fn counter_inherit_source_with_db_null() {
    // counter Inherit + source whose common is db-null → Common::Null.
    let c = counter(
        Common::Inherit,
        Some(CounterSource::Kana(kana_with_common(None))),
    );
    assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Null);
}

#[test]
fn common_proxy_recurses_through_source_chain() {
    let leaf = KaniSimpleTextDispatchEnum::Kanji(kanji_with_common(Some(2)));
    let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
        text: String::new(),
        kana: String::new(),
        source: Box::new(leaf),
        state: SimpleText::default(),
    });
    let outer = ProxyText {
        text: String::new(),
        kana: String::new(),
        source: Box::new(inner),
        state: SimpleText::default(),
    };
    assert_eq!(
        common(&KaniWordDispatchEnum::Proxy(outer)),
        Common::Score(2),
    );
}

#[test]
fn compound_returns_primary_common() {
    let primary = Box::new(KaniWordDispatchEnum::Kanji(kanji_with_common(Some(4))));
    let words = vec![
        KaniWordDispatchEnum::Kanji(kanji_with_common(Some(4))),
        KaniWordDispatchEnum::Kana(kana_with_common(Some(99))),
    ];
    let c = CompoundText {
        text: String::new(),
        kana: String::new(),
        primary,
        words,
        score_base: None,
        score_mod: ScoreMod::Single(0),
    };
    // Compound common reads primary, ignores other words.
    assert_eq!(common(&KaniWordDispatchEnum::Compound(c)), Common::Score(4),);
}

// --- nokanji ---
fn kanji_with(nokanji: bool) -> KanjiText {
    KanjiText {
        id: 0,
        seq: 0,
        text: String::new(),
        ord: 0,
        common: None,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji,
        best_kana: None,
        state: SimpleText::default(),
    }
}

fn kana_with(nokanji: bool) -> KanaText {
    KanaText {
        id: 0,
        seq: 0,
        text: String::new(),
        ord: 0,
        common: None,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji,
        best_kanji: None,
        state: SimpleText::default(),
    }
}

fn nokanji_counter_with_source(source: Option<CounterSource>) -> Counter {
    Counter::Base(CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: "0".into(),
        number: 0,
        source,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed: Vec::new(),
        foreign: false,
    })
}

#[test]
fn nokanji_proxy_recurses_through_source_chain() {
    let leaf = KaniSimpleTextDispatchEnum::Kana(kana_with(true));
    let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
        text: String::new(),
        kana: String::new(),
        source: Box::new(leaf),
        state: SimpleText::default(),
    });
    let outer = ProxyText {
        text: String::new(),
        kana: String::new(),
        source: Box::new(inner),
        state: SimpleText::default(),
    };
    assert_eq!(nokanji(&KaniWordDispatchEnum::Proxy(outer)), Some(true),);
}

#[test]
fn counter_without_source_is_false() {
    let c = nokanji_counter_with_source(None);
    assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(false));
}

#[test]
fn counter_with_kana_source_propagates_flag() {
    let c = nokanji_counter_with_source(Some(CounterSource::Kana(kana_with(true))));
    assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(true));
}

#[test]
fn counter_with_kanji_source_propagates_flag() {
    let c = nokanji_counter_with_source(Some(CounterSource::Kanji(kanji_with(false))));
    assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(false));
}

// --- seq ---
fn seq_kanji(seq: i32) -> KanjiText {
    KanjiText {
        id: 0,
        seq,
        text: String::new(),
        ord: 0,
        common: None,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji: false,
        best_kana: None,
        state: SimpleText::default(),
    }
}

fn seq_kana(seq: i32) -> KanaText {
    KanaText {
        id: 0,
        seq,
        text: String::new(),
        ord: 0,
        common: None,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji: false,
        best_kanji: None,
        state: SimpleText::default(),
    }
}

fn seq_counter_with_source(source: Option<CounterSource>) -> Counter {
    Counter::Base(CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: "0".into(),
        number: 0,
        source,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed: Vec::new(),
        foreign: false,
    })
}

#[test]
fn slot_reader_kanji() {
    assert_eq!(
        seq(&KaniWordDispatchEnum::Kanji(seq_kanji(123))),
        Some(WordInfoSeq::Single(123)),
    );
}

#[test]
fn slot_reader_kana() {
    assert_eq!(
        seq(&KaniWordDispatchEnum::Kana(seq_kana(456))),
        Some(WordInfoSeq::Single(456)),
    );
}

#[test]
fn seq_proxy_recurses_through_source_chain() {
    // proxy → proxy → kana
    let leaf = KaniSimpleTextDispatchEnum::Kana(seq_kana(789));
    let inner_proxy = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
        text: String::new(),
        kana: String::new(),
        source: Box::new(leaf),
        state: SimpleText::default(),
    });
    let outer = ProxyText {
        text: String::new(),
        kana: String::new(),
        source: Box::new(inner_proxy),
        state: SimpleText::default(),
    };
    assert_eq!(
        seq(&KaniWordDispatchEnum::Proxy(outer)),
        Some(WordInfoSeq::Single(789)),
    );
}

#[test]
fn counter_with_source_returns_source_seq() {
    let c = seq_counter_with_source(Some(CounterSource::Kana(seq_kana(2220330))));
    assert_eq!(
        seq(&KaniWordDispatchEnum::Counter(c)),
        Some(WordInfoSeq::Single(2220330)),
    );
}

#[test]
fn counter_without_source_returns_none() {
    let c = seq_counter_with_source(None);
    assert_eq!(seq(&KaniWordDispatchEnum::Counter(c)), None);
}

#[test]
fn compound_collects_word_seqs() {
    let words = vec![
        KaniWordDispatchEnum::Kanji(seq_kanji(1)),
        KaniWordDispatchEnum::Kana(seq_kana(2)),
        KaniWordDispatchEnum::Kanji(seq_kanji(3)),
    ];
    let primary = Box::new(words[0].clone());
    let c = CompoundText {
        text: String::new(),
        kana: String::new(),
        primary,
        words,
        score_base: None,
        score_mod: ScoreMod::Single(0),
    };
    assert_eq!(
        seq(&KaniWordDispatchEnum::Compound(c)),
        Some(WordInfoSeq::Multi(vec![
            Some(WordInfoSeq::Single(1)),
            Some(WordInfoSeq::Single(2)),
            Some(WordInfoSeq::Single(3)),
        ])),
    );
}

#[test]
fn compound_preserves_sourceless_counter_words_as_nil() {
    // dict.lisp:617 (defmethod seq ((obj compound-text)) (mapcar #'seq (words obj)))
    // — `mapcar` preserves position; the sourceless counter-text
    // contributes a `nil` entry which becomes [`None`] in the Vec.
    let words = vec![
        KaniWordDispatchEnum::Kana(seq_kana(10)),
        KaniWordDispatchEnum::Counter(seq_counter_with_source(None)),
        KaniWordDispatchEnum::Kana(seq_kana(20)),
    ];
    let primary = Box::new(words[0].clone());
    let c = CompoundText {
        text: String::new(),
        kana: String::new(),
        primary,
        words,
        score_base: None,
        score_mod: ScoreMod::Single(0),
    };
    assert_eq!(
        seq(&KaniWordDispatchEnum::Compound(c)),
        Some(WordInfoSeq::Multi(vec![
            Some(WordInfoSeq::Single(10)),
            None,
            Some(WordInfoSeq::Single(20)),
        ])),
    );
}

// --- source ---
fn source_kanji(seq: i32) -> KanjiText {
    KanjiText {
        id: 0,
        seq,
        text: String::new(),
        ord: 0,
        common: None,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji: false,
        best_kana: None,
        state: SimpleText::default(),
    }
}

fn source_kana(seq: i32) -> KanaText {
    KanaText {
        id: 0,
        seq,
        text: String::new(),
        ord: 0,
        common: None,
        common_tags: String::new(),
        conjugate_p: true,
        nokanji: false,
        best_kanji: None,
        state: SimpleText::default(),
    }
}

fn source_counter_with_source(source: Option<CounterSource>) -> Counter {
    Counter::Base(CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: "0".into(),
        number: 0,
        source,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed: Vec::new(),
        foreign: false,
    })
}

#[test]
fn counter_kanji_source() {
    let c = source_counter_with_source(Some(CounterSource::Kanji(source_kanji(42))));
    match source(&KaniWordDispatchEnum::Counter(c)) {
        Some(SourceRef::CounterKanji(k)) => assert_eq!(k.seq, 42),
        other => panic!("expected CounterKanji, got {:?}", other),
    }
}

#[test]
fn counter_kana_source() {
    let c = source_counter_with_source(Some(CounterSource::Kana(source_kana(7))));
    match source(&KaniWordDispatchEnum::Counter(c)) {
        Some(SourceRef::CounterKana(k)) => assert_eq!(k.seq, 7),
        other => panic!("expected CounterKana, got {:?}", other),
    }
}

#[test]
fn counter_no_source_returns_none() {
    let c = source_counter_with_source(None);
    assert!(source(&KaniWordDispatchEnum::Counter(c)).is_none());
}

#[test]
fn proxy_source_borrows_simple_chain() {
    let inner = KaniSimpleTextDispatchEnum::Kanji(source_kanji(99));
    let proxy = ProxyText {
        text: String::new(),
        kana: String::new(),
        source: Box::new(inner),
        state: SimpleText::default(),
    };
    match source(&KaniWordDispatchEnum::Proxy(proxy)) {
        Some(SourceRef::ProxySimple(KaniSimpleTextDispatchEnum::Kanji(k))) => {
            assert_eq!(k.seq, 99);
        }
        other => panic!("expected ProxySimple(Kanji), got {:?}", other),
    }
}

#[test]
fn kanji_kana_compound_have_no_source() {
    assert!(source(&KaniWordDispatchEnum::Kanji(source_kanji(1))).is_none());
    assert!(source(&KaniWordDispatchEnum::Kana(source_kana(1))).is_none());
    // Compound covered by the kani_word dispatcher path; constructing
    // a CompoundText needs its full payload — the variant's no-source
    // arm is exercised via the match-arm coverage of the other
    // tests since it shares the `_ => None` branch.
}

// --- value_string ---
// Unit coverage targets the four dispatch arms at the boundaries
// that distinguish them. Bulk behavioural coverage lives in
// `corpus/extracted_counter_2026_05_08/dict/value_string.parquet`
// replayed by `audit_fixtures`.

fn value_string_base(number: u64, ordinalp: bool, descs: Vec<&str>) -> CounterText {
    CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: number.to_string(),
        number,
        source: None,
        ordinalp,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: descs.into_iter().map(String::from).collect(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed: Vec::new(),
        foreign: false,
    }
}

#[test]
fn default_numeric() {
    let c = Counter::Base(value_string_base(5, false, vec![]));
    assert_eq!(value_string(&c), "Value: 5");
}

#[test]
fn default_ordinal() {
    let c = Counter::Base(value_string_base(2, true, vec![]));
    assert_eq!(value_string(&c), "Value: 2nd");
}

#[test]
fn default_with_descriptions_reversed_and_space_prefixed() {
    // Lisp `~{ ~a~}` over (reverse '("d1" "d2")) → " d2 d1".
    let c = Counter::Base(value_string_base(5, false, vec!["d1", "d2"]));
    assert_eq!(value_string(&c), "Value: 5 d2 d1");
}

#[test]
fn halfhour_emits_n_colon_30() {
    let c = Counter::Halfhour(CounterHalfhour(value_string_base(5, false, vec![])));
    assert_eq!(value_string(&c), "5:30");
}

#[test]
fn months_indexes_into_english_array() {
    let c = Counter::Months(CounterMonths(value_string_base(1, false, vec![])));
    assert_eq!(value_string(&c), "January");
    let c = Counter::Months(CounterMonths(value_string_base(3, false, vec![])));
    assert_eq!(value_string(&c), "March");
    let c = Counter::Months(CounterMonths(value_string_base(12, false, vec![])));
    assert_eq!(value_string(&c), "December");
}

#[test]
fn wari_emits_n_times_10_percent() {
    let c = Counter::Wari(CounterWari(value_string_base(5, false, vec![])));
    assert_eq!(value_string(&c), "50%");
    let c = Counter::Wari(CounterWari(value_string_base(1, false, vec![])));
    assert_eq!(value_string(&c), "10%");
}

// --- verify ---
// Unit coverage targets the three dispatch arms (Tsu, DaysOn,
// default) at the boundary values that distinguish them. Bulk
// behavioural coverage lives in
// `corpus/extracted_counter_2026_05_08/dict/verify.parquet`
// (137,676 rows across 11 variants) replayed by `audit_fixtures`.

fn verify_base(number: u64, allowed: Vec<i32>) -> CounterText {
    CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: number.to_string(),
        number,
        source: None,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed,
        foreign: false,
    }
}

#[test]
fn default_passes_when_allowed_empty() {
    let c = Counter::Base(verify_base(0, vec![]));
    assert!(verify(&c, true));
    assert!(!verify(&c, false));
}

#[test]
fn default_checks_allowed_membership() {
    let c = Counter::Base(verify_base(5, vec![1, 2, 3, 4, 5]));
    assert!(verify(&c, true));
    let c = Counter::Base(verify_base(6, vec![1, 2, 3, 4, 5]));
    assert!(!verify(&c, true));
}

#[test]
fn tsu_in_range() {
    for n in 1..=9 {
        assert!(
            verify(&Counter::Tsu(CounterTsu(verify_base(n, vec![]))), true),
            "n={}",
            n
        );
    }
    assert!(!verify(
        &Counter::Tsu(CounterTsu(verify_base(0, vec![]))),
        true
    ));
    assert!(!verify(
        &Counter::Tsu(CounterTsu(verify_base(10, vec![]))),
        true
    ));
}

#[test]
fn days_on_excludes_20_and_2_through_10() {
    // Valid: n == 1 or n > 10, but not 20.
    assert!(verify(
        &Counter::DaysOn(CounterDaysOn(verify_base(1, vec![]))),
        true
    ));
    assert!(verify(
        &Counter::DaysOn(CounterDaysOn(verify_base(11, vec![]))),
        true
    ));
    assert!(verify(
        &Counter::DaysOn(CounterDaysOn(verify_base(31, vec![]))),
        true
    ));
    // Boundary: 20 is owned by counter-days-kun.
    assert!(!verify(
        &Counter::DaysOn(CounterDaysOn(verify_base(20, vec![]))),
        true
    ));
    // 2..=10 (except 1) are kun-yomi territory.
    for n in 2..=10 {
        assert!(
            !verify(
                &Counter::DaysOn(CounterDaysOn(verify_base(n, vec![]))),
                true
            ),
            "n={}",
            n
        );
    }
}

#[test]
fn days_on_chains_to_default_for_allowed() {
    // call-next-method runs the default; if allowed is set and n
    // doesn't match, the chain returns false even when the days-on
    // gate passed. allowed=NIL is the captured shape; this case
    // pins the behaviour for any future days-on recipe with a list.
    let c = Counter::DaysOn(CounterDaysOn(verify_base(11, vec![1, 11, 31])));
    assert!(verify(&c, true));
    let c = Counter::DaysOn(CounterDaysOn(verify_base(11, vec![1, 31])));
    assert!(!verify(&c, true));
}

#[test]
fn unique_false_overrides_everything() {
    // Default + Tsu + DaysOn all AND with `unique`; unique=false
    // short-circuits.
    assert!(!verify(&Counter::Base(verify_base(0, vec![])), false));
    assert!(!verify(
        &Counter::Tsu(CounterTsu(verify_base(5, vec![]))),
        false
    ));
    assert!(!verify(
        &Counter::DaysOn(CounterDaysOn(verify_base(11, vec![]))),
        false
    ));
}

// --- ordinal_str ---
// Unit tests cover the three teen-band edges plus a sample of
// each digit-suffix case. Bulk behavioural coverage lives in
// `corpus/extracted_counter_2026_05_08/dict/ordinal_str.parquet`
// (134 rows) replayed by `audit_fixtures`.

#[test]
fn small_digits_select_st_nd_rd_th() {
    assert_eq!(ordinal_str(1), "1st");
    assert_eq!(ordinal_str(2), "2nd");
    assert_eq!(ordinal_str(3), "3rd");
    assert_eq!(ordinal_str(4), "4th");
    assert_eq!(ordinal_str(7), "7th");
    assert_eq!(ordinal_str(0), "0th");
}

#[test]
fn teens_force_th() {
    // (mod n 100) ∈ 11..=19 → "th" regardless of last digit.
    assert_eq!(ordinal_str(11), "11th");
    assert_eq!(ordinal_str(12), "12th");
    assert_eq!(ordinal_str(13), "13th");
    assert_eq!(ordinal_str(19), "19th");
    assert_eq!(ordinal_str(111), "111th");
    assert_eq!(ordinal_str(212), "212th");
}

#[test]
fn non_teen_twos_threes_etc_use_digit_suffix() {
    assert_eq!(ordinal_str(21), "21st");
    assert_eq!(ordinal_str(22), "22nd");
    assert_eq!(ordinal_str(23), "23rd");
    assert_eq!(ordinal_str(101), "101st");
    assert_eq!(ordinal_str(122), "122nd");
    assert_eq!(ordinal_str(1000), "1000th");
}

#[test]
fn negative_uses_floor_mod() {
    // Lisp (mod -1 10) = 9, so digit "9" → "th". Format prints "-1".
    assert_eq!(ordinal_str(-1), "-1th");
    // (mod -21 10) = 9; (mod -21 100) = 79 (not in 11..=19) → "th".
    assert_eq!(ordinal_str(-21), "-21th");
}
