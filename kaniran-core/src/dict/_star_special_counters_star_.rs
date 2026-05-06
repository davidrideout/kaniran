//! Port of `ichiran/dict:*special-counters*` (`dict-counters.lisp:211`).
//!
//! Per-seq registry of counter constructors. Lisp init form is
//! `(make-hash-table)` — empty at file load — populated by ~91
//! `def-special-counter` macro callsites scattered through
//! `dict-counters.lisp:382-765`. Each callsite registers a function
//! that, given the readings list for a JMdict seq, yields the
//! [`CounterArgs`] recipes that the `*counter-cache*` populator
//! later stores under each recipe's text key.
//!
//! ## Storage
//!
//! Process-global [`OnceLock`]: the registry is pure compile-time
//! data with no DB access, so eager construction would just defer
//! the same allocation to every `KaniranContext::from_url`. Lazy
//! once-init lets the registry live a single time per process and
//! be shared across every context.
//!
//! ## Closure shape
//!
//! Every registered fn has signature
//! `fn(&[KanjiText], &[KanaText]) -> Vec<CounterArgs>`. Non-capturing
//! closures coerce to this fn-pointer type so the body of
//! [`build_special_counters`] can stay one big block of inline
//! lambdas, mirroring the upstream's flat sequence of macro calls.
//!
//! ## Multi-text expansion
//!
//! Upstream `args` accepts `'(t1 t2 ...)`; we pre-expand via
//! [`super::kani_counter_args::args_multi`] so each emitted
//! [`CounterArgs`] carries a single resolved text + source. The
//! cache populator then iterates a flat vec regardless of whether
//! the upstream form was single- or multi-text.
//!
//! ## Class hierarchy
//!
//! `:digit-set` and `:allowed` only apply to `counter-hifumi`
//! variants in the upstream, which gives us a tiny audit hook —
//! we can later assert these fields are empty unless the class is
//! `Hifumi`.

use crate::dict::counter_text_class::{Common, DigitOp, DigitOp as D, DigitOptKey as K};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_counter_args::{
    args, args_multi, args_suffix, digit_opts, CounterArgs, CounterClass as C,
};
use crate::dict::kani_suffix_kind::SuffixKind;
use crate::dict::kanji_text_dao::KanjiText;
use std::collections::HashMap;
use std::sync::OnceLock;

pub type SpecialCounterFn = fn(kanji: &[KanjiText], kana: &[KanaText]) -> Vec<CounterArgs>;

/// Borrow the per-seq special-counter registry, populating on first
/// call. Identity-stable across calls within a process.
pub fn special_counters() -> &'static HashMap<i32, SpecialCounterFn> {
    static MAP: OnceLock<HashMap<i32, SpecialCounterFn>> = OnceLock::new();
    MAP.get_or_init(build_special_counters)
}

/// Replace shortcut: `:r` style `(digit "literal-kana")` op-list
/// element that overrides the digit's kana to a literal string.
fn rep(s: &str) -> DigitOp {
    DigitOp::Replace(s.to_string())
}

/// Construct the registry. Mirrors the flat sequence of
/// `def-special-counter` callsites in `dict-counters.lisp:382-765`,
/// in source order. Each insert encodes one callsite.
pub fn build_special_counters() -> HashMap<i32, SpecialCounterFn> {
    let mut m: HashMap<i32, SpecialCounterFn> = HashMap::new();

    // (def-special-counter 1203020 () (args 'counter-text "階" "かい" :digit-opts '((3 :r))))
    m.insert(1203020, |kj, kn| {
        vec![args(C::Text, "階", "かい", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    // 2020680: 時/じ digit-opts ((4 "よ") (7 "しち") (9 "く"))
    m.insert(2020680, |kj, kn| {
        vec![args(C::Text, "時", "じ", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(7), &[rep("しち")]),
            (K::Digit(9), &[rep("く")]),
        ]))]
    });

    // 1315920: 時間/じかん ((4 "よ") (9 "く"))
    m.insert(1315920, |kj, kn| {
        vec![args(C::Text, "時間", "じかん", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(9), &[rep("く")]),
        ]))]
    });

    // 1658480: 時半/じはん (counter-halfhour) ((4 "よ") (9 "く"))
    m.insert(1658480, |kj, kn| {
        vec![args(C::Halfhour, "時半", "じはん", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(9), &[rep("く")]),
        ]))]
    });

    // 1356740: 畳/じょう ((4 "よ") (7 "しち"))
    m.insert(1356740, |kj, kn| {
        vec![args(C::Text, "畳", "じょう", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(7), &[rep("しち")]),
        ]))]
    });

    // 2258110: 帖/じょう ((4 "よ") (7 "しち"))
    m.insert(2258110, |kj, kn| {
        vec![args(C::Text, "帖", "じょう", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(7), &[rep("しち")]),
        ]))]
    });

    // 1396490: 膳/ぜん ((4 "よ") (7 "しち"))
    m.insert(1396490, |kj, kn| {
        vec![args(C::Text, "膳", "ぜん", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(7), &[rep("しち")]),
        ]))]
    });

    // 1427240: 丁/ちょう plus suffix 丁目/ちょうめ :ordinalp t
    m.insert(1427240, |kj, kn| {
        vec![
            args(C::Text, "丁", "ちょう", kj, kn),
            args_suffix(C::Text, ("丁", "目"), ("ちょう", "め"), kj, kn).ordinalp(true),
        ]
    });

    // 1427420: 丁目/ちょうめ :ordinalp t
    m.insert(1427420, |kj, kn| {
        vec![args(C::Text, "丁目", "ちょうめ", kj, kn).ordinalp(true)]
    });

    // 1514050: 舗/ほ ((4 :h))
    m.insert(1514050, |kj, kn| {
        vec![args(C::Text, "舗", "ほ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    // 1522150: 本/ほん ((3 :r))
    m.insert(1522150, |kj, kn| {
        vec![args(C::Text, "本", "ほん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    // 1583370: ("匹" "疋")/ひき ((3 :r))
    m.insert(1583370, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(3), &[D::Rendaku])]);
        args_multi(C::Text, &["匹", "疋"], "ひき", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

    // 1607310: 羽/わ ((3 :c "ば") (6 :g :c "ぱ") (10 :g :c "ぱ") (100 :g :c "ぱ") (1000 :c "ば") (10000 :c "ば"))
    m.insert(1607310, |kj, kn| {
        vec![args(C::Text, "羽", "わ", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(3), &[D::Counter, rep("ば")]),
            (K::Digit(6), &[D::Geminate, D::Counter, rep("ぱ")]),
            (K::Digit(10), &[D::Geminate, D::Counter, rep("ぱ")]),
            (K::Digit(100), &[D::Geminate, D::Counter, rep("ぱ")]),
            (K::Digit(1000), &[D::Counter, rep("ば")]),
            (K::Digit(10000), &[D::Counter, rep("ば")]),
        ]))]
    });

    // 1607320: 把/わ ((3 :c "ば") (7 "しち") (10 :g :c "ぱ"))
    m.insert(1607320, |kj, kn| {
        vec![args(C::Text, "把", "わ", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(3), &[D::Counter, rep("ば")]),
            (K::Digit(7), &[rep("しち")]),
            (K::Digit(10), &[D::Geminate, D::Counter, rep("ぱ")]),
        ]))]
    });

    // 1633690: 段/だん ((7 "しち"))
    m.insert(1633690, |kj, kn| {
        vec![args(C::Text, "段", "だん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(7), &[rep("しち")])]))]
    });

    // 1901390: 敗/はい ((4 :h))
    m.insert(1901390, |kj, kn| {
        vec![args(C::Text, "敗", "はい", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    // 1919550: 泊/はく ((4 :h))
    m.insert(1919550, |kj, kn| {
        vec![args(C::Text, "泊", "はく", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    // 1994890: 首/しゅ ((10))
    m.insert(1994890, |kj, kn| {
        vec![args(C::Text, "首", "しゅ", kj, kn).digit_opts(digit_opts(&[(K::Digit(10), &[])]))]
    });

    // 1351270: 章/しょう ((10))
    m.insert(1351270, |kj, kn| {
        vec![args(C::Text, "章", "しょう", kj, kn).digit_opts(digit_opts(&[(K::Digit(10), &[])]))]
    });

    // 2019640: ("杯" "盃")/はい ((3 :r))
    m.insert(2019640, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(3), &[D::Rendaku])]);
        args_multi(C::Text, &["杯", "盃"], "はい", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

    // 2078550: 条/じょう ((7 "しち"))
    m.insert(2078550, |kj, kn| {
        vec![args(C::Text, "条", "じょう", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(7), &[rep("しち")])]))]
    });

    // 2078590: 軒/けん ((3 :r))
    m.insert(2078590, |kj, kn| {
        vec![args(C::Text, "軒", "けん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    // 2081610: ("立て" "たて" "タテ")/たて ((:off))
    m.insert(2081610, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Text, &["立て", "たて", "タテ"], "たて", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

    // 2084840: 年/ねん ((4 "よ") (7 "しち") (9 "く")) :accepts '(:kan)
    m.insert(2084840, |kj, kn| {
        vec![args(C::Text, "年", "ねん", kj, kn)
            .digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(7), &[rep("しち")]),
                (K::Digit(9), &[rep("く")]),
            ]))
            .accepts(vec![SuffixKind::Kan])]
    });

    // 1468900: 年生/ねんせい ((4 "よ") (7 "しち") (9 "く"))
    m.insert(1468900, |kj, kn| {
        vec![args(C::Text, "年生", "ねんせい", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(7), &[rep("しち")]),
            (K::Digit(9), &[rep("く")]),
        ]))]
    });

    // 1502840: 分/ふん ((4 :h))
    m.insert(1502840, |kj, kn| {
        vec![args(C::Text, "分", "ふん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    // 2386360: 分間/ふんかん ((4 :h))
    m.insert(2386360, |kj, kn| {
        vec![args(C::Text, "分間", "ふんかん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    // 1373990: 世紀/せいき ((10 "じっ"))
    m.insert(1373990, |kj, kn| {
        vec![args(C::Text, "世紀", "せいき", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(10), &[rep("じっ")])]))]
    });

    // 2836694: 傑/けつ ((10 "じっ"))
    m.insert(2836694, |kj, kn| {
        vec![args(C::Text, "傑", "けつ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(10), &[rep("じっ")])]))]
    });

    // 2208060: 遍/へん ((3 :r))
    m.insert(2208060, |kj, kn| {
        vec![args(C::Text, "遍", "へん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    // 1511870: ("編" "篇")/へん ((3 :r))
    m.insert(1511870, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(3), &[D::Rendaku])]);
        args_multi(C::Text, &["編", "篇"], "へん", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

    // 2271620: 口/こう
    m.insert(2271620, |kj, kn| vec![args(C::Text, "口", "こう", kj, kn)]);

    // 2412230: 足/そく ((3 :r))
    m.insert(2412230, |kj, kn| {
        vec![args(C::Text, "足", "そく", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    // 1175570: 円/えん ((4 "よ"))
    m.insert(1175570, |kj, kn| {
        vec![args(C::Text, "円", "えん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    // 1315130: 字/じ ((4 "よ"))
    m.insert(1315130, |kj, kn| {
        vec![args(C::Text, "字", "じ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    // 1487770: 筆/ひつ ((4 :h))
    m.insert(1487770, |kj, kn| {
        vec![args(C::Text, "筆", "ひつ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    // 2220330: counter-tsu つ/つ
    m.insert(2220330, |kj, kn| vec![args(C::Tsu, "つ", "つ", kj, kn)]);

    // 1208920: 株/かぶ (counter-hifumi) :digit-set '(1 2)
    m.insert(1208920, |kj, kn| {
        vec![args(C::Hifumi, "株", "かぶ", kj, kn).digit_set(vec![1, 2])]
    });

    // 1214060: ("竿" "棹")/さお :digit-set '(1 2 3 4 5) :digit-opts '((4 "よ") (10))
    m.insert(1214060, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(4), &[rep("よ")]), (K::Digit(10), &[])]);
        args_multi(C::Hifumi, &["竿", "棹"], "さお", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3, 4, 5]).digit_opts(opts.clone()))
            .collect()
    });

    // 1260670: 本/もと :digit-set '(1 2 3) (uncertain)
    m.insert(1260670, |kj, kn| {
        vec![args(C::Hifumi, "本", "もと", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 1275640: 口/くち :digit-set '(1 2 3)
    m.insert(1275640, |kj, kn| {
        vec![args(C::Hifumi, "口", "くち", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 1299680: ("皿" "盤")/さら :digit-set '(1 2 3)
    m.insert(1299680, |kj, kn| {
        args_multi(C::Hifumi, &["皿", "盤"], "さら", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]))
            .collect()
    });

    // 1302680: 山/やま :digit-set '(1 2 3) (uncertain)
    m.insert(1302680, |kj, kn| {
        vec![args(C::Hifumi, "山", "やま", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 1335810: ("重ね" "襲")/かさね :digit-set '(1 2 3) (uncertain)
    m.insert(1335810, |kj, kn| {
        args_multi(C::Hifumi, &["重ね", "襲"], "かさね", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]))
            .collect()
    });

    // 1361130: ("振り" "風")/ふり :digit-set '(1 2) :digit-opts '((:off))
    m.insert(1361130, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["振り", "風"], "ふり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    // 1366210: ("針" "鉤" "鈎")/はり :digit-set '(1 2) :digit-opts '((:off))
    m.insert(1366210, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["針", "鉤", "鈎"], "はり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    // 1379650: ("盛り" "盛")/もり :digit-set '(1 2)
    m.insert(1379650, |kj, kn| {
        args_multi(C::Hifumi, &["盛り", "盛"], "もり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    // 1383800: ("切り" "限り" "限")/きり :digit-set '(1 2 3) :digit-opts '((4 "よ") (8))
    m.insert(1383800, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(4), &[rep("よ")]), (K::Digit(8), &[])]);
        args_multi(C::Hifumi, &["切り", "限り", "限"], "きり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]).digit_opts(opts.clone()))
            .collect()
    });

    // 1384840: 切れ/きれ :digit-set '(1 2 3) :digit-opts '((4 "よ") (8))
    m.insert(1384840, |kj, kn| {
        vec![args(C::Hifumi, "切れ", "きれ", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")]), (K::Digit(8), &[])]))]
    });

    // 1385780: 折/おり :digit-set '(1 2)
    m.insert(1385780, |kj, kn| {
        vec![args(C::Hifumi, "折", "おり", kj, kn).digit_set(vec![1, 2])]
    });

    // 1404450: 束/たば :digit-set '(1 2)
    m.insert(1404450, |kj, kn| {
        vec![args(C::Hifumi, "束", "たば", kj, kn).digit_set(vec![1, 2])]
    });

    // 1426480: 柱/はしら :digit-set '(1 2) :digit-opts '((:off))
    m.insert(1426480, |kj, kn| {
        vec![args(C::Hifumi, "柱", "はしら", kj, kn)
            .digit_set(vec![1, 2])
            .digit_opts(digit_opts(&[(K::Off, &[])]))]
    });

    // 1432920: 通り/とおり :digit-set '(1 2) :digit-opts '((100 :g))
    m.insert(1432920, |kj, kn| {
        vec![args(C::Hifumi, "通り", "とおり", kj, kn)
            .digit_set(vec![1, 2])
            .digit_opts(digit_opts(&[(K::Digit(100), &[D::Geminate])]))]
    });

    // 1445150: 度/たび :digit-set '(1 2) :digit-opts '((:off)) :common :null
    m.insert(1445150, |kj, kn| {
        vec![args(C::Hifumi, "度", "たび", kj, kn)
            .digit_set(vec![1, 2])
            .digit_opts(digit_opts(&[(K::Off, &[])]))
            .common(Common::Null)]
    });

    // 1448350: 棟/むね :digit-set '(1 2)
    m.insert(1448350, |kj, kn| {
        vec![args(C::Hifumi, "棟", "むね", kj, kn).digit_set(vec![1, 2])]
    });

    // 1335730: 重/え (let ((digit-set '(1 2 3 5 7 8 9 10))) :digit-set ds :allowed ds)
    m.insert(1335730, |kj, kn| {
        let ds = vec![1, 2, 3, 5, 7, 8, 9, 10];
        vec![args(C::Hifumi, "重", "え", kj, kn)
            .digit_set(ds.clone())
            .allowed(ds)]
    });

    // 2108240: 重/じゅう (counter-text) ((4 "し") (7 "しち") (9 "く"))
    m.insert(2108240, |kj, kn| {
        vec![args(C::Text, "重", "じゅう", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("し")]),
            (K::Digit(7), &[rep("しち")]),
            (K::Digit(9), &[rep("く")]),
        ]))]
    });

    // 1482110: 晩/ばん :digit-set '(1 2 3) :digit-opts '((4 "よ"))
    m.insert(1482110, |kj, kn| {
        vec![args(C::Hifumi, "晩", "ばん", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    // 1501110: ("腹" "肚")/はら :digit-set '(1 2) :digit-opts '((:off))
    m.insert(1501110, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["腹", "肚"], "はら", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    // 1397450: 組 — TWO entries: hifumi (1,2,3) with suffix-descriptions; counter-text with digit-opts ((1))
    m.insert(1397450, |kj, kn| {
        vec![
            args(C::Hifumi, "組", "くみ", kj, kn)
                .digit_set(vec![1, 2, 3])
                .allowed(vec![1, 2, 3])
                .suffix_descriptions(vec!["(sets or pairs only)".to_string()]),
            args(C::Text, "組", "くみ", kj, kn)
                .digit_opts(digit_opts(&[(K::Digit(1), &[])])),
        ]
    });

    // 1519300: ("房" "総")/ふさ :digit-set '(1 2) :digit-opts '((:off))
    m.insert(1519300, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["房", "総"], "ふさ", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    // 1552890: 粒/つぶ :digit-set '(1 2 3) :digit-opts '((6 :g))
    m.insert(1552890, |kj, kn| {
        vec![args(C::Hifumi, "粒", "つぶ", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Digit(6), &[D::Geminate])]))]
    });

    // 1564410: 一刎/はね :digit-set '(1 2 3) :digit-opts '((:off))
    m.insert(1564410, |kj, kn| {
        vec![args(C::Hifumi, "一刎", "はね", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Off, &[])]))]
    });

    // 1585650: ("箱" "函" "匣" "筥" "筐" "凾")/はこ :digit-set '(1 2) :digit-opts '((4 "よ") (1000) (10000))
    m.insert(1585650, |kj, kn| {
        let opts = digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(1000), &[]),
            (K::Digit(10000), &[]),
        ]);
        args_multi(C::Hifumi, &["箱", "函", "匣", "筥", "筐", "凾"], "はこ", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    // 1602800: ("船" "舟")/ふね :digit-set '(1 2 3) :digit-opts '((:off)) (uncertain)
    m.insert(1602800, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["船", "舟"], "ふね", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]).digit_opts(opts.clone()))
            .collect()
    });

    // 1853450: ("締め" "〆")/しめ :digit-set '(1 2) (uncertain)
    m.insert(1853450, |kj, kn| {
        args_multi(C::Hifumi, &["締め", "〆"], "しめ", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    // 1215240: 間/ま :digit-set '(1 2 3 4 9) :digit-opts '((4 "よ"))
    m.insert(1215240, |kj, kn| {
        vec![args(C::Hifumi, "間", "ま", kj, kn)
            .digit_set(vec![1, 2, 3, 4, 9])
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    // 2243700: 咫/あた :digit-set '(1 2 3)
    m.insert(2243700, |kj, kn| {
        vec![args(C::Hifumi, "咫", "あた", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 2414730: 梱/こり :digit-set '(1 2)
    m.insert(2414730, |kj, kn| {
        vec![args(C::Hifumi, "梱", "こり", kj, kn).digit_set(vec![1, 2])]
    });

    // 1583470: 品/しな :digit-set '(1 2 3) :digit-opts '((4 "よ"))
    m.insert(1583470, |kj, kn| {
        vec![args(C::Hifumi, "品", "しな", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    // 1411070: 袋/ふくろ :digit-set '(1 2 3) :digit-opts '((4 "よ") (10 "じっ" :h))
    m.insert(1411070, |kj, kn| {
        vec![args(C::Hifumi, "袋", "ふくろ", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(10), &[rep("じっ"), D::Handakuten]),
            ]))]
    });

    // 2707020: 袋/たい (counter-text) ((10 "じっ"))
    m.insert(2707020, |kj, kn| {
        vec![args(C::Text, "袋", "たい", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(10), &[rep("じっ")])]))]
    });

    // 2800530: ("回り" "廻り")/まわり :digit-set '(1 2)
    m.insert(2800530, |kj, kn| {
        args_multi(C::Hifumi, &["回り", "廻り"], "まわり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    // 1047880: ケース/ケース :digit-set '(1 2) :foreign t
    m.insert(1047880, |kj, kn| {
        vec![args(C::Hifumi, "ケース", "ケース", kj, kn)
            .digit_set(vec![1, 2])
            .foreign(true)]
    });

    // 1214540: 缶/かん :digit-set '(1 2)
    m.insert(1214540, |kj, kn| {
        vec![args(C::Hifumi, "缶", "かん", kj, kn).digit_set(vec![1, 2])]
    });

    // 1575510: ("齣" "コマ")/こま :digit-set '(1 2)
    m.insert(1575510, |kj, kn| {
        args_multi(C::Hifumi, &["齣", "コマ"], "こま", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    // 1253800: 桁/けた :digit-set '(1 2 3)
    m.insert(1253800, |kj, kn| {
        vec![args(C::Hifumi, "桁", "けた", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 1241750: 筋/すじ :digit-set '(1 2 3)
    m.insert(1241750, |kj, kn| {
        vec![args(C::Hifumi, "筋", "すじ", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 1515340: 包み/つつみ :digit-set '(1 2 3)
    m.insert(1515340, |kj, kn| {
        vec![args(C::Hifumi, "包み", "つつみ", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 2452360: 片/ひら :digit-set '(1 2 3)
    m.insert(2452360, |kj, kn| {
        vec![args(C::Hifumi, "片", "ひら", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 2844070: 腰/こし :digit-set '(1 2 3)
    m.insert(2844070, |kj, kn| {
        vec![args(C::Hifumi, "腰", "こし", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 2844196: 緡/さし :digit-set '(1 2 3)
    m.insert(2844196, |kj, kn| {
        vec![args(C::Hifumi, "緡", "さし", kj, kn).digit_set(vec![1, 2, 3])]
    });

    // 1175140: 駅/えき :digit-set '(1 2)
    m.insert(1175140, |kj, kn| {
        vec![args(C::Hifumi, "駅", "えき", kj, kn).digit_set(vec![1, 2])]
    });

    // 2855028: 揃え/そろえ :digit-set '(1 2)
    m.insert(2855028, |kj, kn| {
        vec![args(C::Hifumi, "揃え", "そろえ", kj, kn).digit_set(vec![1, 2])]
    });

    // 2083110: counter-days-kun 日/か :common 0 :accepts '(:kan)
    m.insert(2083110, |kj, kn| {
        vec![args(C::DaysKun, "日", "か", kj, kn)
            .common(Common::Score(0))
            .accepts(vec![SuffixKind::Kan])]
    });

    // 2083100: counter-days-on 日/にち
    m.insert(2083100, |kj, kn| vec![args(C::DaysOn, "日", "にち", kj, kn)]);

    // 1255430: counter-months 月/がつ
    m.insert(1255430, |kj, kn| vec![args(C::Months, "月", "がつ", kj, kn)]);

    // 2149890: counter-people 人/にん ((4 "よ") (7 "しち")) :accepts '(:chuu)
    m.insert(2149890, |kj, kn| {
        vec![args(C::People, "人", "にん", kj, kn)
            .digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(7), &[rep("しち")]),
            ]))
            .accepts(vec![SuffixKind::Chuu])]
    });

    // 1606800: counter-wari 割/わり
    m.insert(1606800, |kj, kn| vec![args(C::Wari, "割", "わり", kj, kn)]);

    // 1606950: counter-wari 割引/わりびき
    m.insert(1606950, |kj, kn| vec![args(C::Wari, "割引", "わりびき", kj, kn)]);

    // 1294940: counter-age ("歳" "才")/さい
    m.insert(1294940, |kj, kn| args_multi(C::Age, &["歳", "才"], "さい", kj, kn));

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build-loop regression (CONVENTIONS §6): the registry should
    /// hold exactly one entry per `def-special-counter` callsite in
    /// upstream `dict-counters.lisp`. Drift here means a duplicate
    /// `m.insert` (silently overwriting) or a missing one.
    #[test]
    fn builds_91_entries_one_per_upstream_callsite() {
        let map = build_special_counters();
        assert_eq!(map.len(), 91, "expected 91 special-counter seqs");
    }

    /// Pin the iteration shape: a registered fn called with empty
    /// readings should still return a valid (possibly source-less)
    /// `Vec<CounterArgs>`. Catches the case where a callsite forgot
    /// to wrap its output in `vec![...]` or returned a wrong type.
    #[test]
    fn every_fn_runs_on_empty_readings() {
        let map = build_special_counters();
        for (seq, f) in &map {
            let out = f(&[], &[]);
            assert!(!out.is_empty(), "seq {} returned no entries", seq);
            for a in &out {
                assert!(!a.text.is_empty(), "seq {}: empty text", seq);
                assert!(!a.kana.is_empty(), "seq {}: empty kana", seq);
                // No source resolves against empty readings.
                assert!(a.source.is_none(), "seq {}: source resolved against empty readings", seq);
            }
        }
    }
}

