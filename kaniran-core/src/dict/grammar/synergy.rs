use crate::dict::grammar::penalty::{synergy_no_toori, synergy_oki};
use crate::dict::kani_lite_segment::{
    KaniLiteSegment, KPCL_C, KPCL_K, KPCL_L, KPCL_P, POS_ADJ_NA, POS_ADJ_NO, POS_ADV_TO, POS_N,
    POS_NOUN,
};
use crate::dict::kani_lite_segment_list::{make_kani_lite_segment_list_from, KaniLiteSegmentList};
use crate::dict::path::SegmentList;
use crate::dict::scoring::score::Segment;
use std::sync::Arc;

/// Port of `ichiran/dict:synergy` (`dict-grammar.lisp:713`).
///
/// In-memory record describing one inter-word scoring bonus applied
/// between two consecutive segments in a parsed path (e.g. noun +
/// particle, na-adjective + な).
// `(defstruct synergy description connector score start end)` has no
// `:initform`s, so every slot defaults to nil. The `description` and
// `connector` slots get bound to strings by most upstream
// `def-generic-synergy` callsites, but a few register synergies that
// leave them nil (encountered in the wi-path bulk corpus). `score`,
// `start`, `end` are always set by the macro expansion to integers.
#[derive(Debug, Clone)]
pub struct Synergy {
    pub description: Option<String>,
    pub connector: Option<String>,
    pub score: i32,
    pub start: usize,
    pub end: usize,
}

/// Port of `ichiran/dict:*synergy-list*` (`dict-grammar.lisp:723`).
///
/// Registry of synergy functions applied to adjacent segment pairs
/// during scoring.
pub type SynergyFn = fn(
    &KaniLiteSegmentList,
    &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)>;

pub static SYNERGY_LIST: &[SynergyFn] = &[
    synergy_oki,
    synergy_no_toori,
    synergy_shika_negative,
    synergy_shicha_ikenai,
    synergy_kanji_prefix,
    synergy_o_prefix,
    synergy_suffix_sei,
    synergy_suffix_buri,
    synergy_suffix_tachi,
    synergy_suffix_chu,
    synergy_to_adverbs,
    synergy_na_adjectives,
    synergy_no_adjectives,
    synergy_sou_nanda,
    synergy_no_da,
    synergy_noun_da,
    synergy_noun_particle,
];

/// Port of `ichiran/dict:make-segment-list-from` (`dict-grammar.lisp:718`).
///
/// Copies a [`SegmentList`] and swaps in a different `segments` vector,
/// carrying over the `start`, `end`, `top`, and `matches` slots verbatim.
pub fn make_segment_list_from(
    old_segment_list: &SegmentList,
    segments: Vec<Arc<Segment>>,
) -> SegmentList {
    // Lisp `copy-segment-list` is a shallow defstruct copy that then
    // gets its segments slot overwritten — the old segments are
    // immediately discarded. Constructing the new struct directly
    // avoids the Rust `Clone` deep-copying the old segments only for
    // them to be replaced on the next line.
    SegmentList {
        segments,
        start: old_segment_list.start,
        end: old_segment_list.end,
        top: old_segment_list.top.clone(),
        matches: old_segment_list.matches,
    }
}

/// Port of `ichiran/dict:def-generic-synergy` (`dict-grammar.lisp:731-746`).
///
/// Shared body for the generic synergy definers: when the two adjacent
/// segment-lists abut and each has segments passing its left/right
/// filter, emit a `(right-list, synergy, left-list)` triple over the
/// filtered segments.
pub struct DefGenericSynergyOpts<'a> {
    pub description: Option<&'a str>,
    pub connector: &'a str,
    pub score: i32,
}

pub fn def_generic_synergy_body(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
    filter_left: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    filter_right: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    opts: &DefGenericSynergyOpts<'_>,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    let start = segment_list_left.end;
    let end = segment_list_right.start;
    // dict-grammar.lisp:737 (when (= start end))
    if start != end {
        return vec![];
    }
    // dict-grammar.lisp:740 (when (and left right)) — tested before
    // materializing either filtered list, so the abut-but-no-match case
    // (one side empties) allocates nothing.
    if !segment_list_left.segments.iter().any(&filter_left)
        || !segment_list_right.segments.iter().any(&filter_right)
    {
        return vec![];
    }
    // dict-grammar.lisp:738-739 (remove-if-not filter-left/right over segment-list-segments)
    let left: Vec<Arc<KaniLiteSegment>> = segment_list_left
        .segments
        .iter()
        .filter(|s| filter_left(s))
        .cloned()
        .collect();
    let right: Vec<Arc<KaniLiteSegment>> = segment_list_right
        .segments
        .iter()
        .filter(|s| filter_right(s))
        .cloned()
        .collect();
    // dict-grammar.lisp:741-746 (list (list (make-segment-list-from r right) (make-synergy ...) (make-segment-list-from l left)))
    let syn = Synergy {
        description: opts.description.map(|d| d.to_string()),
        connector: Some(opts.connector.to_string()),
        score: opts.score,
        start,
        end,
    };
    vec![(
        Arc::new(make_kani_lite_segment_list_from(segment_list_right, right)),
        syn,
        Arc::new(make_kani_lite_segment_list_from(segment_list_left, left)),
    )]
}

/// Port of `ichiran/dict:filter-is-noun` (`dict-grammar.lisp:748`).
///
/// Tests whether a segment is a noun: a kpcl-gated word with one of the
/// six noun parts-of-speech, or a counter-text with a non-empty seq-set.
pub fn filter_is_noun(segment: &Arc<KaniLiteSegment>) -> bool {
    let kpcl = segment.kpcl;
    let kpcl_gate = (kpcl & (KPCL_L | KPCL_K)) != 0 || (kpcl & KPCL_P != 0 && kpcl & KPCL_C != 0);
    if kpcl_gate && (segment.pos & POS_NOUN) != 0 {
        return true;
    }
    segment.is_counter && !segment.seq_set.is_empty()
}

/// Port of `ichiran/dict:filter-is-pos` (`dict-grammar.lisp:757`).
///
/// Returns a segment predicate: the caller's `kpcl_test` body over the
/// `(kanji-or-katakana, primary, common, long)` quad, AND-ed with an
/// overlap between `pos_mask` and the segment's parts-of-speech.
pub fn filter_is_pos(
    pos_mask: u16,
    kpcl_test: impl Fn(bool, bool, bool, bool) -> bool,
) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool {
        let k = segment.kpcl & KPCL_K != 0;
        let p = segment.kpcl & KPCL_P != 0;
        let c = segment.kpcl & KPCL_C != 0;
        let l = segment.kpcl & KPCL_L != 0;
        kpcl_test(k, p, c, l) && (segment.pos & pos_mask) != 0
    }
}

/// Port of `ichiran/dict:filter-in-seq-set` (`dict-grammar.lisp:767`).
///
/// Returns a predicate that tests whether a segment's `:seq-set`
/// shares any seq with the supplied list.
pub fn filter_in_seq_set(seqs: &[i32]) -> impl Fn(&Arc<KaniLiteSegment>) -> bool + '_ {
    move |segment| -> bool { seqs.iter().any(|seq| segment.seq_set.contains(seq)) }
}

/// Port of `ichiran/dict:filter-in-seq-set-simple` (`dict-grammar.lisp:772`).
///
/// Returns a predicate testing whether a segment's word is non-compound
/// (a single seq) AND its `:seq-set` intersects the supplied list.
pub fn filter_in_seq_set_simple(seqs: &[i32]) -> impl Fn(&Arc<KaniLiteSegment>) -> bool + '_ {
    move |segment| -> bool {
        segment.has_simple_seq && seqs.iter().any(|seq| segment.seq_set.contains(seq))
    }
}

/// Port of `ichiran/dict:filter-is-conjugation` (`dict-grammar.lisp:780`).
///
/// Tests whether a segment's `:conj` records include one with the
/// supplied `conj_type`.
pub fn filter_is_conjugation(conj_type: i32) -> impl Fn(&Arc<KaniLiteSegment>) -> bool {
    move |segment| -> bool { segment.conj_types.contains(&conj_type) }
}

/// Port of `ichiran/dict:*noun-particles*` (`dict-grammar.lisp:801`).
///
/// Seqs of particles that can follow a noun. `1005120` appears twice
/// (さえ and すら) — entries are per-meaning-cluster, not per-seq.
pub static NOUN_PARTICLES: &[i32] = &[
    2028920, // は
    2028930, // が
    2028990, // に
    2028980, // で
    2029000, // へ
    1007340, // だけ
    1579080, // ごろ
    1525680, // まで
    2028940, // も
    1582300, // など
    2215430, // には
    1469800, // の
    1009990, // のみ
    2029010, // を
    1005120, // さえ
    2034520, // でさえ
    1005120, // すら
    1008490, // と
    1008530, // とか
    1008590, // として
    2028950, // とは
    2028960, // や
    1009600, // にとって
];

/// Port of `ichiran/dict:filter-is-compound-end` (`dict-grammar.lisp:786`).
///
/// Tests whether a segment's word is a compound whose last child's
/// seq matches any of the supplied seqs.
pub fn filter_is_compound_end(seqs: &[i32]) -> impl Fn(&Arc<KaniLiteSegment>) -> bool + '_ {
    move |segment| -> bool {
        match segment.compound_end_seq {
            Some(end_seq) => seqs.contains(&end_seq),
            None => false,
        }
    }
}

/// Port of `ichiran/dict:filter-is-compound-end-text` (`dict-grammar.lisp:794`).
///
/// Returns a predicate testing whether a segment's word is a compound
/// whose last child's text matches any of the supplied texts.
pub fn filter_is_compound_end_text<'a>(
    texts: &'a [&'a str],
) -> impl Fn(&Arc<KaniLiteSegment>) -> bool + 'a {
    move |segment| -> bool {
        match segment.compound_end_text.as_deref() {
            Some(end) => texts.contains(&end),
            None => false,
        }
    }
}

/// Port of `ichiran/dict:synergy-noun-particle` (`dict-grammar.lisp:827`).
///
/// "noun+prt" synergy: binds a noun on the left to one of the
/// `*noun-particles*` on the right, scoring higher for longer particles.
pub fn synergy_noun_particle(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    // dict-grammar.lisp:831 (:score (+ 10 (* 4 (- (segment-list-end r) (segment-list-start r)))))
    let span = r.end - r.start;
    let score = 10 + 4 * (span as i32);
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(NOUN_PARTICLES),
        &DefGenericSynergyOpts {
            description: Some("noun+prt"),
            connector: " ",
            score,
        },
    )
}

/// Port of `ichiran/dict:synergy-noun-da` (`dict-grammar.lisp:841`).
///
/// "noun+da" synergy: binds a noun on the left to だ (seq 2089020) on the
/// right.
pub fn synergy_noun_da(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(&[2089020]),
        &DefGenericSynergyOpts {
            description: Some("noun+da"),
            connector: " ",
            score: 10,
        },
    )
}

/// Port of `ichiran/dict:synergy-no-da` (`dict-grammar.lisp:848`).
///
/// "no da/desu" synergy: binds の/ん (seqs 1469800/2139720) on the left to
/// だ/です/だろう (seqs 2089020/1007370/1928670) on the right.
pub fn synergy_no_da(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(&[1469800, 2139720]),
        filter_in_seq_set(&[2089020, 1007370, 1928670]),
        &DefGenericSynergyOpts {
            description: Some("no da/desu"),
            connector: " ",
            score: 15,
        },
    )
}

/// Port of `ichiran/dict:synergy-sou-nanda` (`dict-grammar.lisp:856`).
///
/// "sou na n da" synergy: binds そう (seq 2137720) on the left to なんだ
/// (seq 2140410) on the right.
pub fn synergy_sou_nanda(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(&[2137720]),
        filter_in_seq_set(&[2140410]),
        &DefGenericSynergyOpts {
            description: Some("sou na n da"),
            connector: " ",
            score: 50,
        },
    )
}

/// Port of `ichiran/dict:synergy-no-adjectives` (`dict-grammar.lisp:863`).
///
/// "no-adjective" synergy: binds an adj-no on the left to の
/// (seq 1469800) on the right.
pub fn synergy_no_adjectives(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:864 (filter-is-pos ("adj-no") (or k l (and p c)))
        filter_is_pos(POS_ADJ_NO, |k, p, c, l| k || l || (p && c)),
        filter_in_seq_set(&[1469800]),
        &DefGenericSynergyOpts {
            description: Some("no-adjective"),
            connector: " ",
            score: 15,
        },
    )
}

/// Port of `ichiran/dict:synergy-na-adjectives` (`dict-grammar.lisp:870`).
///
/// "na-adjective" synergy: binds an adj-na on the left to な/に
/// (seqs 2029110/2028990) on the right.
pub fn synergy_na_adjectives(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:871 (filter-is-pos ("adj-na") (or k l (and p c)))
        filter_is_pos(POS_ADJ_NA, |k, p, c, l| k || l || (p && c)),
        filter_in_seq_set(&[2029110, 2028990]),
        &DefGenericSynergyOpts {
            description: Some("na-adjective"),
            connector: " ",
            score: 15,
        },
    )
}

/// Port of `ichiran/dict:synergy-to-adverbs` (`dict-grammar.lisp:877`).
///
/// Scores an adv-to word followed by と (seq 1008490), with the score
/// growing by the left span length.
pub fn synergy_to_adverbs(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    // dict-grammar.lisp:881 (:score (+ 10 (* 10 (- (segment-list-end l) (segment-list-start l)))))
    let span = l.end - l.start;
    let score = 10 + 10 * (span as i32);
    def_generic_synergy_body(
        l,
        r,
        // dict-grammar.lisp:878 (filter-is-pos ("adv-to") (or k l p))
        filter_is_pos(POS_ADV_TO, |k, p, _c, l| k || l || p),
        filter_in_seq_set(&[1008490]),
        &DefGenericSynergyOpts {
            description: Some("to-adverb"),
            connector: " ",
            score,
        },
    )
}

/// Port of `ichiran/dict:synergy-suffix-chu` (`dict-grammar.lisp:884`).
///
/// "suffix-chu" synergy: binds a noun on the left to 中 (seqs
/// 1620400/2083570) on the right.
pub fn synergy_suffix_chu(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(&[1620400, 2083570]),
        &DefGenericSynergyOpts {
            description: Some("suffix-chu"),
            connector: "-",
            score: 12,
        },
    )
}

/// Port of `ichiran/dict:synergy-suffix-tachi` (`dict-grammar.lisp:891`).
///
/// Scores a noun on the left followed by the 達 (-tachi) suffix word.
pub fn synergy_suffix_tachi(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(&[1416220]),
        &DefGenericSynergyOpts {
            description: Some("suffix-tachi"),
            connector: "-",
            score: 10,
        },
    )
}

/// Port of `ichiran/dict:synergy-suffix-buri` (`dict-grammar.lisp:898`).
///
/// "suffix-buri" synergy: binds a noun on the left to ぶり (seq 1361140)
/// on the right.
pub fn synergy_suffix_buri(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(&[1361140]),
        &DefGenericSynergyOpts {
            description: Some("suffix-buri"),
            connector: "",
            score: 40,
        },
    )
}

/// Port of `ichiran/dict:synergy-suffix-sei` (`dict-grammar.lisp:905`).
///
/// Scores a noun on the left followed by the 性 (-sei) suffix word.
pub fn synergy_suffix_sei(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_noun,
        filter_in_seq_set(&[1375260]),
        &DefGenericSynergyOpts {
            description: Some("suffix-sei"),
            connector: "",
            score: 12,
        },
    )
}

/// Port of `ichiran/dict:synergy-o-prefix` (`dict-grammar.lisp:913`).
///
/// "o+noun" synergy: binds the honorific お (seq 1270190) on the left to a
/// noun on the right.
pub fn synergy_o_prefix(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(&[1270190]),
        // dict-grammar.lisp:915 (filter-is-pos ("n") (or k l))
        filter_is_pos(POS_N, |k, _p, _c, l| k || l),
        &DefGenericSynergyOpts {
            description: Some("o+noun"),
            connector: "",
            score: 10,
        },
    )
}

/// Port of `ichiran/dict:synergy-kanji-prefix` (`dict-grammar.lisp:920`).
///
/// "kanji prefix+noun" synergy: binds a kanji prefix (未/不, seqs
/// 2242840/1922780/2423740) on the left to a noun on the right.
pub fn synergy_kanji_prefix(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(&[2242840, 1922780, 2423740]),
        // dict-grammar.lisp:922 (filter-is-pos ("n") k)
        filter_is_pos(POS_N, |k, _p, _c, _l| k),
        &DefGenericSynergyOpts {
            description: Some("kanji prefix+noun"),
            connector: "",
            score: 15,
        },
    )
}

/// Port of `ichiran/dict:synergy-shicha-ikenai` (`dict-grammar.lisp:927`).
///
/// "shicha ikenai" synergy: binds a compound ending in は (seq 2028920) on
/// the left to いけない/いけません/だめ/いかん/いや on the right.
pub fn synergy_shicha_ikenai(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_is_compound_end(&[2028920]),
        filter_in_seq_set(&[1000730, 1612750, 1409110, 2829697, 1587610]),
        &DefGenericSynergyOpts {
            description: Some("shicha ikenai"),
            connector: " ",
            score: 50,
        },
    )
}

/// Port of `ichiran/dict:synergy-shika-negative` (`dict-grammar.lisp:934`).
///
/// "shika+neg" synergy: binds しか (seq 1005460) on the left to any
/// segment with a negated conjugation on the right.
pub fn synergy_shika_negative(
    l: &KaniLiteSegmentList,
    r: &KaniLiteSegmentList,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    def_generic_synergy_body(
        l,
        r,
        filter_in_seq_set(&[1005460]),
        // dict-grammar.lisp:936-939 (lambda (some (conj-neg (conj-data-prop cdata)) :conj))
        |s| s.conj_has_neg,
        &DefGenericSynergyOpts {
            description: Some("shika+neg"),
            connector: " ",
            score: 50,
        },
    )
}

#[cfg(test)]
mod tests;
