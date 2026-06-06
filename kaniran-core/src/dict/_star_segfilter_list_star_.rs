//! Port of `ichiran/dict:*segfilter-list*` (`dict-grammar.lisp:1024`).
//!
//! Registry of segfilter functions applied to adjacent segment pairs.

use std::sync::Arc;

use super::segfilter_aux_verb::segfilter_aux_verb;
use super::segfilter_badend::segfilter_badend;
use super::segfilter_dashi::segfilter_dashi;
use super::segfilter_dekiru::segfilter_dekiru;
use super::segfilter_honorific::segfilter_honorific;
use super::segfilter_janai::segfilter_janai;
use super::segfilter_mononi::segfilter_mononi;
use super::segfilter_n::segfilter_n;
use super::segfilter_nohayamete::segfilter_nohayamete;
use super::segfilter_roku::segfilter_roku;
use super::segfilter_sae::segfilter_sae;
use super::segfilter_sukiyoki::segfilter_sukiyoki;
use super::segfilter_toomou::segfilter_toomou;
use super::segfilter_totte::segfilter_totte;
use super::segfilter_tsu_iru::segfilter_tsu_iru;
use super::segfilter_wokarasu::segfilter_wokarasu;
use super::kani_lite_segment_list::KaniLiteSegmentList;

pub type SegFilter = fn(
    Option<&Arc<KaniLiteSegmentList>>,
    &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>;

pub static SEGFILTER_LIST: &[SegFilter] = &[
    segfilter_mononi,
    segfilter_honorific,
    segfilter_dekiru,
    segfilter_dashi,
    segfilter_totte,
    segfilter_toomou,
    segfilter_nohayamete,
    segfilter_janai,
    segfilter_sae,
    segfilter_roku,
    segfilter_sukiyoki,
    segfilter_badend,
    segfilter_wokarasu,
    segfilter_n,
    segfilter_tsu_iru,
    segfilter_aux_verb,
];
