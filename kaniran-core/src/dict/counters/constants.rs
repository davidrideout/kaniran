use crate::dict::counters::classes::{Common, DigitOp, DigitOp as D, DigitOptKey as K};
use crate::dict::counters::kani_counter_args::{
    args, args_multi, args_suffix, digit_opts, CounterArgs, CounterClass as C,
};
use crate::dict::dao::KanaText;
use crate::dict::grammar::suffix::kani_suffix_kind::SuffixKind;
use crate::dict::dao::KanjiText;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `ichiran/dict:*special-counters*` (`dict-counters.lisp:211`).
///
/// seq → counter constructor that, given a JMdict entry's readings,
/// returns the counter-reading recipes for that counter.
pub type SpecialCounterFn = fn(kanji: &[KanjiText], kana: &[KanaText]) -> Vec<CounterArgs>;

pub fn special_counters() -> &'static HashMap<i32, SpecialCounterFn> {
    static MAP: OnceLock<HashMap<i32, SpecialCounterFn>> = OnceLock::new();
    MAP.get_or_init(build_special_counters)
}

fn rep(s: &str) -> DigitOp {
    DigitOp::Replace(s.to_string())
}

pub fn build_special_counters() -> HashMap<i32, SpecialCounterFn> {
    let mut m: HashMap<i32, SpecialCounterFn> = HashMap::new();

    m.insert(1203020, |kj, kn| {
        vec![args(C::Text, "階", "かい", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    m.insert(2020680, |kj, kn| {
        vec![args(C::Text, "時", "じ", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(7), &[rep("しち")]),
            (K::Digit(9), &[rep("く")]),
        ]))]
    });

    m.insert(1315920, |kj, kn| {
        vec![
            args(C::Text, "時間", "じかん", kj, kn).digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(9), &[rep("く")]),
            ])),
        ]
    });

    m.insert(1658480, |kj, kn| {
        vec![
            args(C::Halfhour, "時半", "じはん", kj, kn).digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(9), &[rep("く")]),
            ])),
        ]
    });

    m.insert(1356740, |kj, kn| {
        vec![
            args(C::Text, "畳", "じょう", kj, kn).digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(7), &[rep("しち")]),
            ])),
        ]
    });

    m.insert(2258110, |kj, kn| {
        vec![
            args(C::Text, "帖", "じょう", kj, kn).digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(7), &[rep("しち")]),
            ])),
        ]
    });

    m.insert(1396490, |kj, kn| {
        vec![args(C::Text, "膳", "ぜん", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(7), &[rep("しち")]),
        ]))]
    });

    m.insert(1427240, |kj, kn| {
        vec![
            args(C::Text, "丁", "ちょう", kj, kn),
            args_suffix(C::Text, ("丁", "目"), ("ちょう", "め"), kj, kn).ordinalp(true),
        ]
    });

    m.insert(1427420, |kj, kn| {
        vec![args(C::Text, "丁目", "ちょうめ", kj, kn).ordinalp(true)]
    });

    m.insert(1514050, |kj, kn| {
        vec![args(C::Text, "舗", "ほ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    m.insert(1522150, |kj, kn| {
        vec![args(C::Text, "本", "ほん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    m.insert(1583370, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(3), &[D::Rendaku])]);
        args_multi(C::Text, &["匹", "疋"], "ひき", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

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

    m.insert(1607320, |kj, kn| {
        vec![args(C::Text, "把", "わ", kj, kn).digit_opts(digit_opts(&[
            (K::Digit(3), &[D::Counter, rep("ば")]),
            (K::Digit(7), &[rep("しち")]),
            (K::Digit(10), &[D::Geminate, D::Counter, rep("ぱ")]),
        ]))]
    });

    m.insert(1633690, |kj, kn| {
        vec![args(C::Text, "段", "だん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(7), &[rep("しち")])]))]
    });

    m.insert(1901390, |kj, kn| {
        vec![args(C::Text, "敗", "はい", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    m.insert(1919550, |kj, kn| {
        vec![args(C::Text, "泊", "はく", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    m.insert(1994890, |kj, kn| {
        vec![args(C::Text, "首", "しゅ", kj, kn).digit_opts(digit_opts(&[(K::Digit(10), &[])]))]
    });

    m.insert(1351270, |kj, kn| {
        vec![args(C::Text, "章", "しょう", kj, kn).digit_opts(digit_opts(&[(K::Digit(10), &[])]))]
    });

    m.insert(2019640, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(3), &[D::Rendaku])]);
        args_multi(C::Text, &["杯", "盃"], "はい", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

    m.insert(2078550, |kj, kn| {
        vec![args(C::Text, "条", "じょう", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(7), &[rep("しち")])]))]
    });

    m.insert(2078590, |kj, kn| {
        vec![args(C::Text, "軒", "けん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    m.insert(2081610, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Text, &["立て", "たて", "タテ"], "たて", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

    m.insert(2084840, |kj, kn| {
        vec![args(C::Text, "年", "ねん", kj, kn)
            .digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(7), &[rep("しち")]),
                (K::Digit(9), &[rep("く")]),
            ]))
            .accepts(vec![SuffixKind::Kan])]
    });

    m.insert(1468900, |kj, kn| {
        vec![
            args(C::Text, "年生", "ねんせい", kj, kn).digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(7), &[rep("しち")]),
                (K::Digit(9), &[rep("く")]),
            ])),
        ]
    });

    m.insert(1502840, |kj, kn| {
        vec![args(C::Text, "分", "ふん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    m.insert(2386360, |kj, kn| {
        vec![args(C::Text, "分間", "ふんかん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    m.insert(1373990, |kj, kn| {
        vec![args(C::Text, "世紀", "せいき", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(10), &[rep("じっ")])]))]
    });

    m.insert(2836694, |kj, kn| {
        vec![args(C::Text, "傑", "けつ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(10), &[rep("じっ")])]))]
    });

    m.insert(2208060, |kj, kn| {
        vec![args(C::Text, "遍", "へん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    m.insert(1511870, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(3), &[D::Rendaku])]);
        args_multi(C::Text, &["編", "篇"], "へん", kj, kn)
            .into_iter()
            .map(|a| a.digit_opts(opts.clone()))
            .collect()
    });

    m.insert(2271620, |kj, kn| vec![args(C::Text, "口", "こう", kj, kn)]);

    m.insert(2412230, |kj, kn| {
        vec![args(C::Text, "足", "そく", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(3), &[D::Rendaku])]))]
    });

    m.insert(1175570, |kj, kn| {
        vec![args(C::Text, "円", "えん", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    m.insert(1315130, |kj, kn| {
        vec![args(C::Text, "字", "じ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    m.insert(1487770, |kj, kn| {
        vec![args(C::Text, "筆", "ひつ", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(4), &[D::Handakuten])]))]
    });

    m.insert(2220330, |kj, kn| vec![args(C::Tsu, "つ", "つ", kj, kn)]);

    m.insert(1208920, |kj, kn| {
        vec![args(C::Hifumi, "株", "かぶ", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(1214060, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(4), &[rep("よ")]), (K::Digit(10), &[])]);
        args_multi(C::Hifumi, &["竿", "棹"], "さお", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3, 4, 5]).digit_opts(opts.clone()))
            .collect()
    });

    m.insert(1260670, |kj, kn| {
        vec![args(C::Hifumi, "本", "もと", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(1275640, |kj, kn| {
        vec![args(C::Hifumi, "口", "くち", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(1299680, |kj, kn| {
        args_multi(C::Hifumi, &["皿", "盤"], "さら", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]))
            .collect()
    });

    m.insert(1302680, |kj, kn| {
        vec![args(C::Hifumi, "山", "やま", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(1335810, |kj, kn| {
        args_multi(C::Hifumi, &["重ね", "襲"], "かさね", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]))
            .collect()
    });

    m.insert(1361130, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["振り", "風"], "ふり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    m.insert(1366210, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["針", "鉤", "鈎"], "はり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    m.insert(1379650, |kj, kn| {
        args_multi(C::Hifumi, &["盛り", "盛"], "もり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    m.insert(1383800, |kj, kn| {
        let opts = digit_opts(&[(K::Digit(4), &[rep("よ")]), (K::Digit(8), &[])]);
        args_multi(C::Hifumi, &["切り", "限り", "限"], "きり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]).digit_opts(opts.clone()))
            .collect()
    });

    m.insert(1384840, |kj, kn| {
        vec![args(C::Hifumi, "切れ", "きれ", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(8), &[]),
            ]))]
    });

    m.insert(1385780, |kj, kn| {
        vec![args(C::Hifumi, "折", "おり", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(1404450, |kj, kn| {
        vec![args(C::Hifumi, "束", "たば", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(1426480, |kj, kn| {
        vec![args(C::Hifumi, "柱", "はしら", kj, kn)
            .digit_set(vec![1, 2])
            .digit_opts(digit_opts(&[(K::Off, &[])]))]
    });

    m.insert(1432920, |kj, kn| {
        vec![args(C::Hifumi, "通り", "とおり", kj, kn)
            .digit_set(vec![1, 2])
            .digit_opts(digit_opts(&[(K::Digit(100), &[D::Geminate])]))]
    });

    m.insert(1445150, |kj, kn| {
        vec![args(C::Hifumi, "度", "たび", kj, kn)
            .digit_set(vec![1, 2])
            .digit_opts(digit_opts(&[(K::Off, &[])]))
            .common(Common::Null)]
    });

    m.insert(1448350, |kj, kn| {
        vec![args(C::Hifumi, "棟", "むね", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(1335730, |kj, kn| {
        let ds = vec![1, 2, 3, 5, 7, 8, 9, 10];
        vec![args(C::Hifumi, "重", "え", kj, kn)
            .digit_set(ds.clone())
            .allowed(ds)]
    });

    m.insert(2108240, |kj, kn| {
        vec![
            args(C::Text, "重", "じゅう", kj, kn).digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("し")]),
                (K::Digit(7), &[rep("しち")]),
                (K::Digit(9), &[rep("く")]),
            ])),
        ]
    });

    m.insert(1482110, |kj, kn| {
        vec![args(C::Hifumi, "晩", "ばん", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    m.insert(1501110, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["腹", "肚"], "はら", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    m.insert(1397450, |kj, kn| {
        vec![
            args(C::Hifumi, "組", "くみ", kj, kn)
                .digit_set(vec![1, 2, 3])
                .allowed(vec![1, 2, 3])
                .suffix_descriptions(vec!["(sets or pairs only)".to_string()]),
            args(C::Text, "組", "くみ", kj, kn).digit_opts(digit_opts(&[(K::Digit(1), &[])])),
        ]
    });

    m.insert(1519300, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["房", "総"], "ふさ", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
            .collect()
    });

    m.insert(1552890, |kj, kn| {
        vec![args(C::Hifumi, "粒", "つぶ", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Digit(6), &[D::Geminate])]))]
    });

    m.insert(1564410, |kj, kn| {
        vec![args(C::Hifumi, "一刎", "はね", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Off, &[])]))]
    });

    m.insert(1585650, |kj, kn| {
        let opts = digit_opts(&[
            (K::Digit(4), &[rep("よ")]),
            (K::Digit(1000), &[]),
            (K::Digit(10000), &[]),
        ]);
        args_multi(
            C::Hifumi,
            &["箱", "函", "匣", "筥", "筐", "凾"],
            "はこ",
            kj,
            kn,
        )
        .into_iter()
        .map(|a| a.digit_set(vec![1, 2]).digit_opts(opts.clone()))
        .collect()
    });

    m.insert(1602800, |kj, kn| {
        let opts = digit_opts(&[(K::Off, &[])]);
        args_multi(C::Hifumi, &["船", "舟"], "ふね", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2, 3]).digit_opts(opts.clone()))
            .collect()
    });

    m.insert(1853450, |kj, kn| {
        args_multi(C::Hifumi, &["締め", "〆"], "しめ", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    m.insert(1215240, |kj, kn| {
        vec![args(C::Hifumi, "間", "ま", kj, kn)
            .digit_set(vec![1, 2, 3, 4, 9])
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    m.insert(2243700, |kj, kn| {
        vec![args(C::Hifumi, "咫", "あた", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(2414730, |kj, kn| {
        vec![args(C::Hifumi, "梱", "こり", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(1583470, |kj, kn| {
        vec![args(C::Hifumi, "品", "しな", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[(K::Digit(4), &[rep("よ")])]))]
    });

    m.insert(1411070, |kj, kn| {
        vec![args(C::Hifumi, "袋", "ふくろ", kj, kn)
            .digit_set(vec![1, 2, 3])
            .digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(10), &[rep("じっ"), D::Handakuten]),
            ]))]
    });

    m.insert(2707020, |kj, kn| {
        vec![args(C::Text, "袋", "たい", kj, kn)
            .digit_opts(digit_opts(&[(K::Digit(10), &[rep("じっ")])]))]
    });

    m.insert(2800530, |kj, kn| {
        args_multi(C::Hifumi, &["回り", "廻り"], "まわり", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    m.insert(1047880, |kj, kn| {
        vec![args(C::Hifumi, "ケース", "ケース", kj, kn)
            .digit_set(vec![1, 2])
            .foreign(true)]
    });

    m.insert(1214540, |kj, kn| {
        vec![args(C::Hifumi, "缶", "かん", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(1575510, |kj, kn| {
        args_multi(C::Hifumi, &["齣", "コマ"], "こま", kj, kn)
            .into_iter()
            .map(|a| a.digit_set(vec![1, 2]))
            .collect()
    });

    m.insert(1253800, |kj, kn| {
        vec![args(C::Hifumi, "桁", "けた", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(1241750, |kj, kn| {
        vec![args(C::Hifumi, "筋", "すじ", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(1515340, |kj, kn| {
        vec![args(C::Hifumi, "包み", "つつみ", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(2452360, |kj, kn| {
        vec![args(C::Hifumi, "片", "ひら", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(2844070, |kj, kn| {
        vec![args(C::Hifumi, "腰", "こし", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(2844196, |kj, kn| {
        vec![args(C::Hifumi, "緡", "さし", kj, kn).digit_set(vec![1, 2, 3])]
    });

    m.insert(1175140, |kj, kn| {
        vec![args(C::Hifumi, "駅", "えき", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(2855028, |kj, kn| {
        vec![args(C::Hifumi, "揃え", "そろえ", kj, kn).digit_set(vec![1, 2])]
    });

    m.insert(2083110, |kj, kn| {
        vec![args(C::DaysKun, "日", "か", kj, kn)
            .common(Common::Score(0))
            .accepts(vec![SuffixKind::Kan])]
    });

    m.insert(2083100, |kj, kn| {
        vec![args(C::DaysOn, "日", "にち", kj, kn)]
    });

    m.insert(1255430, |kj, kn| {
        vec![args(C::Months, "月", "がつ", kj, kn)]
    });

    m.insert(2149890, |kj, kn| {
        vec![args(C::People, "人", "にん", kj, kn)
            .digit_opts(digit_opts(&[
                (K::Digit(4), &[rep("よ")]),
                (K::Digit(7), &[rep("しち")]),
            ]))
            .accepts(vec![SuffixKind::Chuu])]
    });

    m.insert(1606800, |kj, kn| vec![args(C::Wari, "割", "わり", kj, kn)]);

    m.insert(1606950, |kj, kn| {
        vec![args(C::Wari, "割引", "わりびき", kj, kn)]
    });

    m.insert(1294940, |kj, kn| {
        args_multi(C::Age, &["歳", "才"], "さい", kj, kn)
    });

    m
}

/// Port of `ichiran/dict:*counter-suffixes*` (`dict-counters.lisp:213`).
///
/// Definition table for the three counter-trailing suffixes a counter
/// can advertise via its `:accepts` list. Each entry is
/// `(kind, kanji surface, kana, English description)`.
pub static COUNTER_SUFFIXES: &[(SuffixKind, &str, &str, &str)] = &[
    (SuffixKind::Kan, "間", "かん", "[duration]"),
    (SuffixKind::Kango, "間後", "かんご", "[after ...]"),
    (SuffixKind::Chuu, "中", "ちゅう", "[among/out of ...]"),
];

/// Port of `ichiran/dict:*counter-accepts*` (`dict-counters.lisp:217`).
///
/// Per-seq override of which counter suffixes a JMdict entry accepts.
/// The default is "no suffixes"; entries listed here advertise the
/// given subset of [`crate::dict::grammar::suffix::kani_suffix_kind::SuffixKind`].
pub static COUNTER_ACCEPTS: &[(i32, &[SuffixKind])] = &[
    (1194480, &[SuffixKind::Kan]),
    (1490430, &[SuffixKind::Kan]),
    (1333450, &[SuffixKind::Kan, SuffixKind::Kango]),
];

/// Port of `ichiran/dict:*counter-foreign*` (`dict-counters.lisp:219`).
///
/// Seqs of counter entries treated as foreign, so their katakana
/// readings are pulled as additional sources. One seq today: 1120410
/// (頁/ページ).
pub static COUNTER_FOREIGN: &[i32] = &[1120410];

/// Port of `ichiran/dict:*extra-counter-ids*` (`dict-counters.lisp:310`).
///
/// JMdict seqs treated as counters even though JMdict itself does not
/// tag them with `pos=ctr`.
pub static EXTRA_COUNTER_IDS: &[i32] = &[
    1255430, // 月
    1606800, // 割
];

/// Port of `ichiran/dict:*skip-counter-ids*` (`dict-counters.lisp:315`).
///
/// Seqs JMdict tags as `pos=ctr` but that are excluded from the
/// counter cache (ambiguous readings, mahjong terms with no canonical
/// reading).
pub static SKIP_COUNTER_IDS: &[i32] = &[
    2426510, // 一個当り
    2220370, // 歳 （とせ）
    2248360, // 入 （しお）
    2423450, // 差し
    2671670, // 幅 （の）
    2735690, // 種 （くさ）
    2838543, // 杯 （はた）
    // mahjong stuff - need some research on how to say these
    2249290, // 荘
    2833260, // 翻
    2833465, // 萬
    2833466, // 索
    2833467, // 筒
];

#[cfg(test)]
mod tests;
