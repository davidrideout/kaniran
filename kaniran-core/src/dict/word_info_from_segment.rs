//! Port of `ichiran/dict:word-info-from-segment` (`dict.lisp:1327`).
//!
//! Lifts a scored [`Segment`] into a [`WordInfo`], branching on the
//! segment's word: simple-text fills `true_text` / `conjugations`,
//! compound-text fills `components`, counter-text fills `counter`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::CompoundText;
use crate::dict::get_kana::get_kana;
use crate::dict::get_text::get_text;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::segment_struct::Segment;
use crate::dict::counters::methods::seq;
use crate::dict::true_text::true_text;
use crate::dict::counters::methods::value_string;
use crate::dict::word_conjugations::word_conjugations;
use crate::dict::word_info_class::{WordInfo, WordInfoKana, WordInfoSeq, WordInfoType};
use crate::dict::word_type::{word_type, WordType};

pub async fn word_info_from_segment(
    ctx: &KaniranContext,
    segment: &mut Segment,
) -> Result<WordInfo, sqlx::Error> {
    // dict.lisp:1330 (:text (get-text segment)) — lazy memoization via segment.text
    let text = segment.get_text().to_string();
    // dict.lisp:1347-1348 (:score / :start / :end) — read before re-borrowing word
    let score = segment.score;
    let start = segment.start;
    let end = segment.end;
    let word = &segment.word;

    // dict.lisp:1329 (:type (word-type word))
    let kind = word_info_type_from(word_type(word));

    // dict.lisp:1331 (:kana (get-kana word))
    let kana = get_kana(ctx, word).await?.map(WordInfoKana::Single);

    // dict.lisp:1332 (:seq (seq word))
    let seq_value = seq(word);

    // dict.lisp:1333-1334 (:conjugations / :true-text — gated on simple-text)
    let (true_text_v, conjugations_v) = match word {
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Proxy(_) => (
            Some(true_text(word).into_owned()),
            word_conjugations(word),
        ),
        _ => (None, None),
    };

    // dict.lisp:1335-1345 (:components — gated on compound-text)
    let components = if let KaniWordDispatchEnum::Compound(c) = word {
        compound_components(ctx, c).await?
    } else {
        Vec::new()
    };

    // dict.lisp:1346 (:counter — gated on counter-text)
    let counter = if let KaniWordDispatchEnum::Counter(c) = word {
        Some((value_string(c), c.base().ordinalp))
    } else {
        None
    };

    Ok(WordInfo {
        kind,
        text,
        true_text: true_text_v,
        kana,
        seq: seq_value,
        conjugations: conjugations_v,
        score,
        components,
        counter,
        start: Some(start),
        end: Some(end),
        ..Default::default()
    })
}

async fn compound_components(
    ctx: &KaniranContext,
    c: &CompoundText,
) -> Result<Vec<WordInfo>, sqlx::Error> {
    // dict.lisp:1336 (with primary-seq = (seq (primary word))) — bound once.
    // Lisp's `(= int int)` is the only branch that returns a bool; any
    // non-int operand raises TYPE-ERROR. The Rust port panics on the
    // first non-Single encounter to mirror that.
    let primary_seq = match seq(&c.primary) {
        Some(WordInfoSeq::Single(s)) => s,
        other => panic!(
            "compound-text primary seq must be Single int — Lisp `(= … {:?})` would type-error",
            other
        ),
    };
    let mut out = Vec::with_capacity(c.words.len());
    for wrd in &c.words {
        let wrd_seq = seq(wrd);
        let wrd_seq_int = match wrd_seq.as_ref() {
            Some(WordInfoSeq::Single(s)) => *s,
            other => panic!(
                "compound child seq must be Single int — Lisp `(= {:?} {})` would type-error",
                other, primary_seq
            ),
        };
        let child_kana = get_kana(ctx, wrd)
            .await?
            .map(WordInfoKana::Single);
        out.push(WordInfo {
            // dict.lisp:1339 (:type (word-type wrd))
            kind: word_info_type_from(word_type(wrd)),
            // dict.lisp:1340 (:text (get-text wrd))
            text: get_text(wrd).into_owned(),
            // dict.lisp:1341 (:true-text (true-text wrd))
            true_text: Some(true_text(wrd).into_owned()),
            // dict.lisp:1342 (:kana (get-kana wrd))
            kana: child_kana,
            // dict.lisp:1343 (:seq (seq wrd))
            seq: wrd_seq,
            // dict.lisp:1344 (:conjugations (word-conjugations wrd))
            conjugations: word_conjugations(wrd),
            // dict.lisp:1345 (:primary (= (seq wrd) primary-seq))
            primary: wrd_seq_int == primary_seq,
            ..Default::default()
        });
    }
    Ok(out)
}

fn word_info_type_from(word_type: WordType) -> WordInfoType {
    match word_type {
        WordType::Kanji => WordInfoType::Kanji,
        WordType::Kana => WordInfoType::Kana,
        WordType::Gap => WordInfoType::Gap,
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! Each test exercises one branch of the segment-word dispatch and
    //! confirms the slot mapping against REPL-captured ground truth.
    use super::*;
    use crate::dict::counters::dispatchers::find_counter;
    use crate::dict::find_word::{find_word, FindWordRows};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false).await.unwrap();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .unwrap_or_else(|| panic!("no kanji rows for {word:?}")),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .unwrap_or_else(|| panic!("no kana rows for {word:?}")),
        }
    }

    fn segment(word: KaniWordDispatchEnum, score: i32, start: usize, end: usize) -> Segment {
        Segment {
            start,
            end,
            word,
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    #[tokio::test]
    async fn kana_text_segment_populates_simple_text_slots() {
        // REPL: (find-word-full "ねこ") → KANA-TEXT seq=1467640
        //   word-info-from-segment with score=16, end=2 →
        //   type=KANA text=ねこ kana=ねこ true-text=ねこ primary=T counter=NIL
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "ねこ").await;
        let mut seg = segment(word, 16, 0, 2);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("ねこ".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1467640)));
        assert_eq!(wi.score, Some(16));
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
        assert_eq!(wi.true_text.as_deref(), Some("ねこ"));
        assert!(wi.counter.is_none());
        assert!(wi.primary);
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert!(wi.components.is_empty());
    }

    #[tokio::test]
    async fn kanji_text_segment_returns_text_and_seq() {
        // KANJI-TEXT branch — get-kana goes through best-kana-conj /
        // get-kanji-kana-old / hint dispatch, all live against the DB.
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "猫").await;
        let mut seg = segment(word, 3, 0, 1);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "猫");
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(2698030)));
        assert_eq!(wi.score, Some(3));
        assert_eq!(wi.true_text.as_deref(), Some("猫"));
        assert!(wi.counter.is_none());
        assert!(matches!(wi.kana, Some(WordInfoKana::Single(_))));
    }

    #[tokio::test]
    async fn counter_text_segment_populates_counter_pair_and_null_true_text() {
        // REPL: (word-info-from-segment) on a COUNTER-TEXT "5個":
        //   type=KANJI text=5個 counter=("Value: 5" NIL) true-text=NIL
        let ctx = ctx_from_env().await;
        let counter = find_counter(&ctx, "5", "個", None)
            .into_iter()
            .next()
            .expect("find_counter(5, 個) returned no counters");
        let word = KaniWordDispatchEnum::Counter(counter);
        let mut seg = segment(word, 40, 0, 2);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "5個");
        assert_eq!(wi.counter, Some(("Value: 5".into(), false)));
        assert!(wi.true_text.is_none()); // counter-text is not simple-text
        assert!(wi.conjugations.is_none());
        assert_eq!(wi.score, Some(40));
    }

    #[tokio::test]
    async fn segment_with_no_score_passes_none_through() {
        // The upstream slot is unset (initform 0 only fires when :score
        // initarg is absent — but every callsite supplies :score from
        // (segment-score segment), even if nil).
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "ねこ").await;
        let mut seg = Segment {
            start: 0,
            end: 2,
            word,
            score: None,
            info: None,
            top: None,
            text: None,
        };
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.score, None);
    }

    #[tokio::test]
    async fn compound_text_segment_builds_components_with_primary_flag() {
        // dict.lisp:1335-1345 — compound-text branch:
        //   components = each child word-info with primary set iff
        //   (= (seq wrd) (seq (primary word))).
        use crate::dict::compound_text_class::{CompoundText, ScoreMod};
        let ctx = ctx_from_env().await;
        let w1 = first_reading(&ctx, "ねこ").await; // seq=1467640
        let w2 = first_reading(&ctx, "いぬ").await; // seq=1258330
        let compound = CompoundText {
            text: "ねこいぬ".into(),
            kana: "ねこいぬ".into(),
            primary: Box::new(w1.clone()),
            words: vec![w1, w2],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut seg = segment(KaniWordDispatchEnum::Compound(compound), 5, 0, 4);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.text, "ねこいぬ");
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1467640)),
                Some(WordInfoSeq::Single(1258330)),
            ]))
        );
        assert_eq!(wi.components.len(), 2);
        assert!(wi.components[0].primary); // matches primary's seq
        assert!(!wi.components[1].primary); // different seq
    }
}
