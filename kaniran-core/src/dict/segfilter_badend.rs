//! Port of `ichiran/dict:segfilter-badend` (`dict-grammar.lisp:1095`).
//!
//! ```lisp
//! (def-segfilter-must-follow segfilter-badend (l r)
//!   (constantly nil)
//!   (filter-is-compound-end-text "ちゃい" "いか" "とか" "とき" "い"))
//! ```

use std::sync::Arc;

use super::classify::classify;
use super::filter_is_compound_end_text::filter_is_compound_end_text;
use super::make_segment_list_from::make_segment_list_from;
use super::segment_list_struct::SegmentList;

fn badend_texts() -> Vec<String> {
    vec![
        "ちゃい".to_string(),
        "いか".to_string(),
        "とか".to_string(),
        "とき".to_string(),
        "い".to_string(),
    ]
}

pub fn segfilter_badend(
    seg_left: Option<&Arc<SegmentList>>,
    seg_right: &Arc<SegmentList>,
) -> Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> {
    let filter_right = filter_is_compound_end_text(badend_texts());
    let (sat_r, con_r) = classify(filter_right, &seg_right.segments);

    // Cond clause 1: (or (not sat-r) (and allow-first (not l))).
    // allow-first = nil here, so just (not sat-r).
    if sat_r.is_empty() {
        return vec![(seg_left.cloned(), Arc::clone(seg_right))];
    }

    // Cond clause 2: (or (not l) (/= l.end r.start)).
    let l = match seg_left {
        None => {
            return if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(None, Arc::new(make_segment_list_from(seg_right, con_r)))]
            };
        }
        Some(l) if l.end != seg_right.start => {
            return if con_r.is_empty() {
                Vec::new()
            } else {
                vec![(Some(Arc::clone(l)), Arc::new(make_segment_list_from(seg_right, con_r)))]
            };
        }
        Some(l) => l,
    };

    // T branch. Left filter (constantly nil): sat-l is always empty,
    // con-l is always all of l's segments. con-l is therefore always
    // non-empty when l has segments, so the inner `if con-l` is
    // effectively a tautology here — but the macro template keeps it
    // for the general case, so the port mirrors it.
    let (sat_l, con_l) = classify(|_s| false, &l.segments);

    if con_l.is_empty() {
        // Reachable only if l.segments is empty — rare but possible at
        // upstream call sites that don't filter empty seglists.
        return vec![(Some(Arc::clone(l)), Arc::clone(seg_right))];
    }

    let mut result: Vec<(Option<Arc<SegmentList>>, Arc<SegmentList>)> = Vec::new();
    if !con_r.is_empty() {
        result.push((Some(Arc::clone(l)), Arc::new(make_segment_list_from(seg_right, con_r))));
    }
    if !sat_l.is_empty() {
        // Unreachable for this segfilter — filter-left = (constantly
        // nil) makes sat-l empty. Kept structurally for parity with
        // the macro template.
        result.insert(
            0,
            (
                Some(Arc::new(make_segment_list_from(l, sat_l))),
                Arc::new(make_segment_list_from(seg_right, sat_r)),
            ),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_struct::Segment;
    use crate::dict::simple_text_class::SimpleText;

    fn kana(text: &str, seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn compound(child_texts: &[&str]) -> KaniWordDispatchEnum {
        let words: Vec<KaniWordDispatchEnum> = child_texts
            .iter()
            .enumerate()
            .map(|(i, t)| KaniWordDispatchEnum::Kana(kana(t, 9900 + i as i32)))
            .collect();
        let primary = Box::new(words[0].clone());
        KaniWordDispatchEnum::Compound(CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        })
    }

    fn seg(start: usize, end: usize, word: KaniWordDispatchEnum) -> Segment {
        Segment {
            start,
            end,
            word,
            score: None,
            info: None,
            top: None,
            text: None,
        }
    }

    fn sl(start: usize, end: usize, segments: Vec<Segment>) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }
    }

    // REPL probes from `/tmp/probe_badend.lisp` (this session).

    #[test]
    fn ba_a_l_nil_r_all_match_returns_empty() {
        // Ba-A l=NIL r=all-match -> {} (allow-first=nil, con-r empty)
        let seg_chai = seg(1, 2, compound(&["ちゃい"]));
        let r = Arc::new(sl(1, 2, vec![seg_chai]));
        let result = segfilter_badend(None, &r);
        assert!(result.is_empty());
    }

    #[test]
    fn ba_b_l_nil_r_mixed() {
        // Ba-B l=NIL r=mixed -> {(L=NIL R=[1 seg = non-matching])}
        let seg_chai = seg(1, 2, compound(&["ちゃい"]));
        let seg_x = seg(1, 2, compound(&["x"]));
        let r = Arc::new(sl(1, 2, vec![seg_chai, seg_x]));
        let result = segfilter_badend(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_c_l_nil_r_no_match() {
        // Ba-C l=NIL r=no-match -> {(L=NIL R=r unchanged)} (clause-1)
        let seg_x = seg(1, 2, compound(&["x"]));
        let r = Arc::new(sl(1, 2, vec![seg_x]));
        let result = segfilter_badend(None, &r);
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_none());
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_d_l_adj_r_mixed_emits_base_pair_only() {
        // Ba-D l-adj r=mixed -> {(L=l unchanged 1 seg, R=mslf(r, con_r)=1 seg)}
        // sat-l is always empty (constantly nil) so no prepend.
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_chai = seg(1, 3, compound(&["ちゃい"]));
        let seg_x = seg(1, 3, compound(&["x"]));
        let l = Arc::new(sl(0, 1, vec![seg_simp]));
        let r = Arc::new(sl(1, 3, vec![seg_chai, seg_x]));
        let result = segfilter_badend(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_e_l_adj_r_all_match_empty_result() {
        // Ba-E l-adj r=all-match -> {} (sat-l empty + con-r empty → both branches contribute nothing)
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_chai = seg(1, 3, compound(&["ちゃい"]));
        let l = Arc::new(sl(0, 1, vec![seg_simp]));
        let r = Arc::new(sl(1, 3, vec![seg_chai]));
        let result = segfilter_badend(Some(&l), &r);
        assert!(result.is_empty());
    }

    #[test]
    fn ba_f_l_adj_gap_r_mixed() {
        // Ba-F l-adj-gap (l.end=1, r.start=2) r=mixed ->
        //   {(L=l unchanged, R=mslf(r, con_r)=1 seg)}
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_chai = seg(2, 4, compound(&["ちゃい"]));
        let seg_x = seg(2, 4, compound(&["x"]));
        let l = Arc::new(sl(0, 1, vec![seg_simp]));
        let r = Arc::new(sl(2, 4, vec![seg_chai, seg_x]));
        let result = segfilter_badend(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }

    #[test]
    fn ba_g_l_adj_r_no_match() {
        // Ba-G l-adj r=no-match -> {(L=l unchanged, R=r unchanged)} (clause-1)
        let seg_simp = seg(0, 1, KaniWordDispatchEnum::Kana(kana("い", 9995)));
        let seg_x = seg(1, 3, compound(&["x"]));
        let l = Arc::new(sl(0, 1, vec![seg_simp]));
        let r = Arc::new(sl(1, 3, vec![seg_x]));
        let result = segfilter_badend(Some(&l), &r);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.as_ref().unwrap().segments.len(), 1);
        assert_eq!(result[0].1.segments.len(), 1);
    }
}
