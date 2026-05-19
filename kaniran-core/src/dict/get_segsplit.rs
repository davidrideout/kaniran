//! Port of `ichiran/dict:get-segsplit` (`dict-split.lisp:784`).
//!
//! When `segment.word` is a `simple-text`, dispatch through
//! `*segsplit-map*` for a split that decomposes the reading; on a hit,
//! wrap the parts in a `compound-text`, copy the segment, and overwrite
//! word/text/score/info on the copy. Returns `None` for non-`simple-text`
//! words and for readings with no matching segsplit entry.
//!
//! Diverges from the upstream lambda list `(segment)` by taking
//! `&KaniranContext` per CONVENTIONS §4.8; the special `*split-map*`
//! rebind at `dict-split.lisp:786` becomes the
//! [`KaniranContext::with_segsplit_map`] sibling-context call per
//! CONVENTIONS §4.10. The Lisp `(values split attrs)` pair from
//! `get-split` is collapsed in the Rust [`super::get_split::get_split`]
//! to `(parts, score-int)` — the keyword-arg portion of `attrs`
//! (`:primary`, `:connector`, `:root`) is recovered here by walking
//! [`SEGSPLIT_TABLE`] in the same seq order `get-split*` does
//! (`dict-split.lisp:70-75`).

use crate::characters::join::join;
use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_segsplit_map_star_::{SegSplitDef, SEGSPLIT_TABLE};
use crate::dict::calc_score::calc_score;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::get_kana::get_kana;
use crate::dict::get_split::get_split;
use crate::dict::get_text::get_text;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::segment_struct::{
    KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment,
};
use crate::dict::set_word_conjugations::set_word_conjugations;
use crate::dict::simple_text_class::WordConjugations;
use crate::dict::word_conj_data::word_conj_data;

pub async fn get_segsplit(
    ctx: &KaniranContext,
    segment: &Segment,
) -> Result<Option<Segment>, sqlx::Error> {
    // dict-split.lisp:785 (when (typep word 'simple-text))
    let simple_word = match &segment.word {
        KaniWordDispatchEnum::Kanji(k) => KaniSimpleTextDispatchEnum::Kanji(k.clone()),
        KaniWordDispatchEnum::Kana(k) => KaniSimpleTextDispatchEnum::Kana(k.clone()),
        KaniWordDispatchEnum::Proxy(p) => KaniSimpleTextDispatchEnum::Proxy(p.clone()),
        KaniWordDispatchEnum::Compound(_) | KaniWordDispatchEnum::Counter(_) => {
            return Ok(None);
        }
    };

    // dict-split.lisp:786 (let ((*split-map* *segsplit-map*)) …)
    let ctx2 = ctx.with_segsplit_map();

    // dict-split.lisp:788 (cdr (getf (segment-info segment) :seq-set))
    let conj_of: Vec<i32> = segment
        .info
        .as_ref()
        .map(|info| info.seq_set.iter().skip(1).copied().collect())
        .unwrap_or_default();

    // dict-split.lisp:787-788
    // (multiple-value-bind (split attrs) (get-split word conj-of) …)
    let Some((parts, score)) = get_split(&ctx2, &simple_word, &conj_of).await? else {
        return Ok(None);
    };

    // dict-split.lisp:790 (destructuring-bind (score &key (primary 0) (connector " ") root) attrs)
    // attrs == the score-arg list passed to `def-simple-split` (e.g.
    // '(-10 :root (1))). The Rust SEGSPLIT_TABLE stores the destructured
    // slots alongside SplitDef; recover them by re-walking the same seq
    // order get-split* used.
    let Some(def) = find_segsplit_def(simple_word.seq(), &conj_of) else {
        return Ok(None);
    };

    let mut words: Vec<KaniWordDispatchEnum> = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            SplitPart::Word(w) => words.push(w),
            // SEGSPLIT_TABLE has no Step::Push rows — the registered
            // segsplit forms at dict-split.lisp:706-782 never pass `:score`
            // / `:pscore` as parts. (setf word-conjugations) on a keyword
            // would signal no-applicable-method upstream.
            SplitPart::Score | SplitPart::PScore => unreachable!(
                "segsplit-map split returned :score / :pscore part — not in SEGSPLIT_TABLE shape"
            ),
        }
    }

    // dict-split.lisp:799-801 (when root (loop for i from 0 for word in split
    //                            if (find i root) do (setf (word-conjugations word) :root)))
    if !def.root.is_empty() {
        for (i, w) in words.iter_mut().enumerate() {
            if def.root.contains(&i) {
                set_word_conjugations(w, Some(WordConjugations::Root));
            }
        }
    }

    // dict-split.lisp:793 (:text (join "" (mapcar 'get-text split)))
    let texts: Vec<String> = words.iter().map(|w| get_text(w).into_owned()).collect();
    // dict-split.lisp:794 (:kana (join connector (mapcar 'get-kana split)))
    let mut kanas: Vec<String> = Vec::with_capacity(words.len());
    for w in &words {
        // dict.lisp:638 precedent — `(concatenate 'string nil ...)` treats
        // nil as empty; `Option<String>` from get_kana mirrors with
        // `.unwrap_or_default()`.
        kanas.push(get_kana(ctx, w).await?.unwrap_or_default());
    }

    // dict-split.lisp:795 (:primary (elt split primary))
    let primary_word = Box::new(words[def.primary].clone());

    // dict-split.lisp:791-797 (make-instance 'compound-text …)
    let compound = CompoundText {
        text: join("", &texts),
        kana: join(def.connector, &kanas),
        primary: primary_word,
        words,
        // dict.lisp:613 — `score-base :initform nil :initarg :score-base`;
        // not passed by get-segsplit, slot keeps default nil.
        score_base: None,
        // dict-split.lisp:797 (:score-mod score) — `score` here is the
        // integer head of the attrs list (the destructured `score`
        // positional), held in SplitDef.score.
        score_mod: ScoreMod::Single(score as i64),
    };

    let wrapped = KaniWordDispatchEnum::Compound(compound);

    // dict-split.lisp:805 (nth-value 1 (calc-score (primary word)))
    let primary_ref: &KaniWordDispatchEnum = match &wrapped {
        KaniWordDispatchEnum::Compound(c) => &c.primary,
        _ => unreachable!(),
    };
    let (_disc_score, info_opt) =
        calc_score(ctx, primary_ref, false, None, None, &[]).await?;

    // dict-split.lisp:806 (getf (segment-info new-seg) :conj) (word-conj-data word)
    let conj_data = word_conj_data(ctx, &wrapped).await?;
    // CL setf-getf-on-nil idiom: when `info` is nil, `(setf (getf nil :conj) X)`
    // rewrites the binding to a fresh plist `(:conj X)`. Mirror by
    // synthesizing the same zero/empty KaniSegmentInfo calc_score uses
    // for the analogous compound-recursion path (calc_score.rs:238-250).
    let mut info = info_opt.unwrap_or_else(|| KaniSegmentInfo {
        posi: Vec::new(),
        seq_set: Vec::new(),
        conj: Vec::new(),
        common: None,
        score_info: KaniScoreInfo {
            prop_score: 0,
            kanji_break: Vec::new(),
            use_length_bonus: 0,
            split_info: KaniSplitInfo::None,
        },
        kpcl: (false, false, false, false),
    });
    info.conj = conj_data;

    // dict-split.lisp:798 (new-seg (copy-segment segment))
    let mut new_seg = segment.clone();

    // dict-split.lisp:802-807 — parallel setf populates new-seg slots.
    // Compute reads before the moves so the source values come from a
    // consistent snapshot, matching the parallel-setf semantics.
    let new_text = get_text(&wrapped).into_owned();
    let old_score = segment
        .score
        .expect("get-segsplit: segment.score must be set by gen-score before call");

    new_seg.word = wrapped;
    new_seg.text = Some(new_text);
    new_seg.score = Some(old_score + score);
    new_seg.info = Some(info);

    Ok(Some(new_seg))
}

fn find_segsplit_def(seq: i32, conj_of: &[i32]) -> Option<&'static SegSplitDef> {
    // dict-split.lisp:70 — first lookup is (gethash (seq reading) *split-map*).
    if let Some(def) = SEGSPLIT_TABLE.iter().find(|d| d.split.seq == seq) {
        return Some(def);
    }
    // dict-split.lisp:73-75 — then walk conj-of in order, returning the
    // first match.
    for &s in conj_of {
        if let Some(def) = SEGSPLIT_TABLE.iter().find(|d| d.split.seq == s) {
            return Some(def);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::gen_score::gen_score;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// Helper: find the reading for `text` at `seq`, gen-score it, return the segment.
    async fn segment_for_seq(
        ctx: &KaniranContext,
        text: &str,
        seq: i32,
    ) -> Segment {
        let rows = find_word(ctx, text, false).await.unwrap();
        let word = match rows {
            FindWordRows::Kana(v) => v
                .into_iter()
                .find(|k| k.seq == seq)
                .map(KaniWordDispatchEnum::Kana)
                .unwrap_or_else(|| panic!("no kana row for {} seq={}", text, seq)),
            FindWordRows::Kanji(v) => v
                .into_iter()
                .find(|k| k.seq == seq)
                .map(KaniWordDispatchEnum::Kanji)
                .unwrap_or_else(|| panic!("no kanji row for {} seq={}", text, seq)),
        };
        let mut seg = Segment {
            start: 0,
            end: text.chars().count(),
            word,
            score: None,
            info: None,
            top: None,
            text: Some(text.into()),
        };
        gen_score(ctx, &mut seg, false, &[]).await.unwrap();
        seg
    }

    fn assert_compound(seg: &Segment) -> &CompoundText {
        match &seg.word {
            KaniWordDispatchEnum::Compound(c) => c,
            other => panic!("expected compound-text segment, got {:?}", other),
        }
    }

    fn word_seq(w: &KaniWordDispatchEnum) -> i32 {
        match w {
            KaniWordDispatchEnum::Kanji(k) => k.seq,
            KaniWordDispatchEnum::Kana(k) => k.seq,
            _ => panic!("expected simple-text word in segsplit parts"),
        }
    }

    fn word_text(w: &KaniWordDispatchEnum) -> &str {
        match w {
            KaniWordDispatchEnum::Kanji(k) => &k.text,
            KaniWordDispatchEnum::Kana(k) => &k.text,
            _ => panic!("expected simple-text word in segsplit parts"),
        }
    }

    // REPL probe (2026-05-18, .103):
    //   (ichi:make-segment :word #1343110_kana :text "ところで") + gen-score
    //   pre-score=175, post-score=165, score-mod=-10
    //   text="ところで", kana="ところ で", primary-seq=1343100
    //   words=((1343100 "ところ" nil) (2028980 "で" :root))
    //   info=(:posi (adv n suf) :seq-set (1343100) :conj nil :common 0
    //         :score-info (16 nil 0 nil) :kpcl (nil t t nil))
    #[tokio::test]
    async fn tokorode_root_keyword_marks_index_1() {
        let ctx = ctx_from_env().await;
        let mut seg = segment_for_seq(&ctx, "ところで", 1343110).await;
        // Pin non-default start/end/top so the copy-segment assertion
        // catches a regression that zeroes them inside get-segsplit.
        seg.start = 11;
        seg.end = 15;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let result = get_segsplit(&ctx, &seg).await.unwrap();
        let new_seg = result.expect("ところで matches split-tokorode (segsplit-map)");

        // dict-split.lisp:798 — copy-segment clones start/end/top; the
        // parallel setf only writes word/text/score/info.
        assert_eq!(new_seg.start, 11);
        assert_eq!(new_seg.end, 15);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "ところで");
        assert_eq!(compound.kana, "ところ で");
        assert_eq!(word_seq(&compound.primary), 1343100);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 1343100);
        assert_eq!(word_text(&compound.words[0]), "ところ");
        assert_eq!(word_seq(&compound.words[1]), 2028980);
        assert_eq!(word_text(&compound.words[1]), "で");
        // :root (1) — index-1 word gets WordConjugations::Root.
        assert!(matches!(
            crate::dict::word_conjugations::word_conjugations(&compound.words[1]),
            Some(WordConjugations::Root)
        ));
        assert!(crate::dict::word_conjugations::word_conjugations(&compound.words[0]).is_none());
        assert!(matches!(compound.score_mod, ScoreMod::Single(-10)));
        assert!(compound.score_base.is_none());

        // pre 175 + (-10) = 165 (REPL).
        assert_eq!(new_seg.score, Some(pre_score - 10));
        assert_eq!(new_seg.text.as_deref(), Some("ところで"));
        let info = new_seg.info.as_ref().expect("segsplit info plist non-nil");
        // calc-score on primary (1343100) — full info plist per REPL.
        assert_eq!(info.posi, vec!["adv", "n", "suf"]);
        assert_eq!(info.seq_set, vec![1343100]);
        // word-conj-data on compound = word-conj-data on last word (で root) — REPL nil.
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // REPL probe (2026-05-18, .103):
    //   ところでは reading 1897510 — split-tokorodewa 3-part split
    //   pre-score=275, post-score=265, score-mod=-10
    //   text="ところでは", kana="ところ で ‌は" (U+200C hint mod prefix on は)
    //   primary-seq=1343100 (index 0)
    //   words=((1343100 "ところ") (2028980 "で") (2028920 "は"))
    //   info=(:posi (adv n suf) :seq-set (1343100) :conj nil :common 0
    //         :score-info (16 nil 0 nil) :kpcl (nil t t nil))
    #[tokio::test]
    async fn tokorodewa_three_part_split_default_attrs() {
        let ctx = ctx_from_env().await;
        let mut seg = segment_for_seq(&ctx, "ところでは", 1897510).await;
        seg.start = 3;
        seg.end = 8;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg).await.unwrap().expect("segsplit hit");

        // copy-segment preserves start/end/top.
        assert_eq!(new_seg.start, 3);
        assert_eq!(new_seg.end, 8);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "ところでは");
        assert_eq!(compound.kana, "ところ で ‌は");
        assert_eq!(compound.words.len(), 3);
        assert_eq!(word_seq(&compound.words[0]), 1343100);
        assert_eq!(word_seq(&compound.words[1]), 2028980);
        assert_eq!(word_seq(&compound.words[2]), 2028920);
        // No :root keyword → no word gets marked Root.
        for w in &compound.words {
            assert!(crate::dict::word_conjugations::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(-10)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score - 10));
        assert_eq!(new_seg.text.as_deref(), Some("ところでは"));
        assert_eq!(word_seq(&compound.primary), 1343100);

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["adv", "n", "suf"]);
        assert_eq!(info.seq_set, vec![1343100]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // REPL probe (2026-05-18, .103):
    //   お店 reading 2409240 (kanji-text) — split-omise
    //   pre-score=40, post-score=60, score-mod=20, :primary 1 :connector ""
    //   text="お店", kana="おみせ" (empty connector concats parts)
    //   words=((2826528 "お") (1582120 "店"))
    //   primary-seq=1582120 (index 1 per :primary 1)
    //   info=(:posi (n) :seq-set (1582120) :conj nil :common 0
    //         :score-info (16 nil 0 nil) :kpcl (t t t nil))
    #[tokio::test]
    async fn omise_primary_and_connector_keywords() {
        let ctx = ctx_from_env().await;
        let mut seg = segment_for_seq(&ctx, "お店", 2409240).await;
        seg.start = 4;
        seg.end = 6;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg).await.unwrap().expect("segsplit hit");

        assert_eq!(new_seg.start, 4);
        assert_eq!(new_seg.end, 6);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "お店");
        assert_eq!(compound.kana, "おみせ");
        assert_eq!(word_seq(&compound.primary), 1582120);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 2826528);
        assert_eq!(word_seq(&compound.words[1]), 1582120);
        for w in &compound.words {
            assert!(crate::dict::word_conjugations::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(20)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score + 20));
        assert_eq!(new_seg.text.as_deref(), Some("お店"));

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["n"]);
        assert_eq!(info.seq_set, vec![1582120]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, false));
    }

    // REPL probe (2026-05-18, .103):
    //   だから reading 1007310 — split-dakara, default attrs
    //   pre-score=144, post-score=139, score-mod=-5
    //   text="だから", kana="だ から" (default connector " ")
    //   words=((2089020 "だ") (1002980 "から"))
    //   primary-seq=2089020 (index 0)
    //   info=(:posi (aux-v cop cop-da) :seq-set (2089020) :conj nil :common 0
    //         :score-info (16 nil 0 nil) :kpcl (nil t t nil))
    #[tokio::test]
    async fn dakara_basic_default_attrs() {
        let ctx = ctx_from_env().await;
        let mut seg = segment_for_seq(&ctx, "だから", 1007310).await;
        seg.start = 2;
        seg.end = 5;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg).await.unwrap().expect("segsplit hit");

        assert_eq!(new_seg.start, 2);
        assert_eq!(new_seg.end, 5);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "だから");
        assert_eq!(compound.kana, "だ から");
        assert_eq!(word_seq(&compound.primary), 2089020);
        assert_eq!(word_seq(&compound.words[0]), 2089020);
        assert_eq!(word_seq(&compound.words[1]), 1002980);
        assert!(matches!(compound.score_mod, ScoreMod::Single(-5)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score - 5));
        assert_eq!(new_seg.text.as_deref(), Some("だから"));
        // No :root — neither word marked.
        for w in &compound.words {
            assert!(crate::dict::word_conjugations::word_conjugations(w).is_none());
        }

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["aux-v", "cop", "cop-da"]);
        assert_eq!(info.seq_set, vec![2089020]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 16);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // REPL probe (2026-05-18, .103):
    //   から元気 reading 1675330 (kanji-text) — :primary 1
    //   pre-score=315, post-score=325, score-mod=10
    //   text="から元気", kana="から げんき"
    //   words=((KANA-TEXT 1002980 "から") (KANJI-TEXT 1260720 "元気"))
    //   primary-seq=1260720 (index 1)
    //   info=(:posi (adj-na n) :seq-set (1260720) :conj nil :common 6
    //         :score-info (20 nil 0 nil) :kpcl (t t t nil))
    #[tokio::test]
    async fn karagenki_kanji_input_primary_1() {
        let ctx = ctx_from_env().await;
        let mut seg = segment_for_seq(&ctx, "から元気", 1675330).await;
        seg.start = 6;
        seg.end = 10;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg).await.unwrap().expect("segsplit hit");

        assert_eq!(new_seg.start, 6);
        assert_eq!(new_seg.end, 10);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "から元気");
        assert_eq!(compound.kana, "から げんき");
        assert_eq!(word_seq(&compound.primary), 1260720);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 1002980);
        assert_eq!(word_seq(&compound.words[1]), 1260720);
        for w in &compound.words {
            assert!(crate::dict::word_conjugations::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(10)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score + 10));
        assert_eq!(new_seg.text.as_deref(), Some("から元気"));

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["adj-na", "n"]);
        assert_eq!(info.seq_set, vec![1260720]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(6));
        assert_eq!(info.score_info.prop_score, 20);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (true, true, true, false));
    }

    // REPL probe (2026-05-18, .103):
    //   猫 readings (1467640, 2698030) — neither in segsplit-map → nil.
    #[tokio::test]
    async fn neko_no_segsplit() {
        let ctx = ctx_from_env().await;
        let seg = segment_for_seq(&ctx, "猫", 1467640).await;
        let result = get_segsplit(&ctx, &seg).await.unwrap();
        assert!(result.is_none(), "猫 (1467640) is not in *segsplit-map*");
    }

    // REPL probe (2026-05-18, .103):
    //   はぐったり reading 10494835 — synthetic conjugated form of seq 1010105
    //   (はぐる, archaic), conj-of=(1010105). Exercises the get-split*
    //   conj-of-fallback dispatch branch (dict-split.lisp:73-75) — the
    //   direct-seq tests in this file all match on simple_word.seq().
    //   pre-score=176, post-score=181, score-mod=5
    //   text="はぐったり", kana="‌は ぐったり" (U+200C hint mod prefix on は)
    //   primary-seq=2028920 (index 0, default :primary 0)
    //   words=((2028920 "は") (1004070 "ぐったり"))
    //   info=(:posi (prt) :seq-set (2028920) :conj nil :common 0
    //         :score-info (11 nil 0 nil) :kpcl (nil t t nil))
    #[tokio::test]
    async fn hagguttari_conj_of_fallback_dispatch() {
        let ctx = ctx_from_env().await;
        let mut seg = segment_for_seq(&ctx, "はぐったり", 10494835).await;
        seg.start = 9;
        seg.end = 14;
        seg.top = None;
        let pre_score = seg.score.expect("gen-score sets score");
        let new_seg = get_segsplit(&ctx, &seg).await.unwrap().expect("segsplit hit via conj-of");

        assert_eq!(new_seg.start, 9);
        assert_eq!(new_seg.end, 14);
        assert!(new_seg.top.is_none());

        let compound = assert_compound(&new_seg);
        assert_eq!(compound.text, "はぐったり");
        assert_eq!(compound.kana, "‌は ぐったり");
        assert_eq!(word_seq(&compound.primary), 2028920);
        assert_eq!(compound.words.len(), 2);
        assert_eq!(word_seq(&compound.words[0]), 2028920);
        assert_eq!(word_text(&compound.words[0]), "は");
        assert_eq!(word_seq(&compound.words[1]), 1004070);
        assert_eq!(word_text(&compound.words[1]), "ぐったり");
        for w in &compound.words {
            assert!(crate::dict::word_conjugations::word_conjugations(w).is_none());
        }
        assert!(matches!(compound.score_mod, ScoreMod::Single(5)));
        assert!(compound.score_base.is_none());
        assert_eq!(new_seg.score, Some(pre_score + 5));
        assert_eq!(new_seg.text.as_deref(), Some("はぐったり"));

        let info = new_seg.info.as_ref().expect("info set");
        assert_eq!(info.posi, vec!["prt"]);
        assert_eq!(info.seq_set, vec![2028920]);
        assert!(info.conj.is_empty());
        assert_eq!(info.common, Some(0));
        assert_eq!(info.score_info.prop_score, 11);
        assert!(info.score_info.kanji_break.is_empty());
        assert_eq!(info.score_info.use_length_bonus, 0);
        assert!(matches!(info.score_info.split_info, KaniSplitInfo::None));
        assert_eq!(info.kpcl, (false, true, true, false));
    }

    // dict-split.lisp:785 — (typep word 'simple-text) is false for
    // compound-text; get-segsplit returns nil immediately.
    #[tokio::test]
    async fn compound_text_input_returns_none() {
        use crate::dict::kana_text_dao::KanaText;
        use crate::dict::simple_text_class::SimpleText;
        let ctx = ctx_from_env().await;
        let kana = KanaText {
            id: 0,
            seq: 1,
            text: "ところ".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        };
        let inner = KaniWordDispatchEnum::Kana(kana);
        let compound = CompoundText {
            text: "ところで".into(),
            kana: "ところで".into(),
            primary: Box::new(inner.clone()),
            words: vec![inner.clone(), inner],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let seg = Segment {
            start: 0,
            end: 4,
            word: KaniWordDispatchEnum::Compound(compound),
            score: Some(100),
            info: Some(KaniSegmentInfo {
                posi: Vec::new(),
                seq_set: vec![1343110],
                conj: Vec::new(),
                common: None,
                score_info: KaniScoreInfo {
                    prop_score: 0,
                    kanji_break: Vec::new(),
                    use_length_bonus: 0,
                    split_info: KaniSplitInfo::None,
                },
                kpcl: (false, false, false, false),
            }),
            top: None,
            text: Some("ところで".into()),
        };
        let result = get_segsplit(&ctx, &seg).await.unwrap();
        assert!(result.is_none(), "compound-text input fails typep check");
    }

    // dict-split.lisp:798 — copy-segment preserves start / end / top from
    // the source. The setf parallel-setf only writes word / text / score /
    // info; everything else stays as-cloned from the source segment.
    #[tokio::test]
    async fn copy_segment_preserves_start_end_top() {
        let ctx = ctx_from_env().await;
        let mut seg = segment_for_seq(&ctx, "だから", 1007310).await;
        // Re-anchor at non-default start/end so the assertion catches
        // any drift (gen-score doesn't touch start/end).
        seg.start = 7;
        seg.end = 10;
        seg.top = None;
        let new_seg = get_segsplit(&ctx, &seg).await.unwrap().unwrap();
        assert_eq!(new_seg.start, 7);
        assert_eq!(new_seg.end, 10);
        assert!(new_seg.top.is_none());
    }
}
