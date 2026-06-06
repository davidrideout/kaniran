//! Port of `ichiran/dict:*synergy-list*` (`dict-grammar.lisp:723`).
//!
//! Registry of synergy functions applied to adjacent segment pairs
//! during scoring.

use std::sync::Arc;

use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_kanji_prefix::synergy_kanji_prefix;
use super::synergy_na_adjectives::synergy_na_adjectives;
use super::synergy_no_adjectives::synergy_no_adjectives;
use super::synergy_no_da::synergy_no_da;
use super::synergy_no_toori::synergy_no_toori;
use super::synergy_noun_da::synergy_noun_da;
use super::synergy_noun_particle::synergy_noun_particle;
use super::synergy_o_prefix::synergy_o_prefix;
use super::synergy_oki::synergy_oki;
use super::synergy_shicha_ikenai::synergy_shicha_ikenai;
use super::synergy_shika_negative::synergy_shika_negative;
use super::synergy_sou_nanda::synergy_sou_nanda;
use super::synergy_struct::Synergy;
use super::synergy_suffix_buri::synergy_suffix_buri;
use super::synergy_suffix_chu::synergy_suffix_chu;
use super::synergy_suffix_sei::synergy_suffix_sei;
use super::synergy_suffix_tachi::synergy_suffix_tachi;
use super::synergy_to_adverbs::synergy_to_adverbs;

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
