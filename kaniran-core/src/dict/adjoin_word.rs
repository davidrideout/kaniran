//! Port of `ichiran/dict:adjoin-word` (`dict.lisp:632`).
//!
//! Builds a compound word from `word1` and `word2`: when `word1` is a
//! simple text it mints a fresh `compound-text`; when it is already a
//! compound it extends that compound (concatenating text/kana and
//! growing `score-mod`). Defaults `text`/`kana` to the concatenation
//! of the two inputs and `score-mod` to `0`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::get_kana::get_kana;
use crate::dict::get_text::get_text;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

pub async fn adjoin_word(
    ctx: &KaniranContext,
    word1: KaniWordDispatchEnum,
    word2: KaniSimpleTextDispatchEnum,
    text: Option<String>,
    kana: Option<String>,
    score_mod: Option<ScoreMod>,
    score_base: Option<KaniWordDispatchEnum>,
) -> Result<CompoundText, sqlx::Error> {
    // dict.lisp:635-640 (defmethod adjoin-word :around (t t))
    let resolved_text = match text {
        Some(t) => t,
        None => {
            let word2_as_word = word2.to_word();
            let t1 = get_text(&word1);
            let t2 = get_text(&word2_as_word);
            format!("{}{}", t1, t2)
        }
    };
    let resolved_kana = match kana {
        Some(k) => k,
        None => {
            let word2_as_word = word2.to_word();
            // dict.lisp:638 — (concatenate 'string (get-kana word1) (get-kana word2)).
            // Upstream `(concatenate 'string nil ...)` accepts nil as
            // the empty sequence; the Rust `Option<String>` from
            // `get_kana` mirrors that with `.unwrap_or_default()`.
            let k1 = get_kana(ctx, &word1).await?.unwrap_or_default();
            let k2 = get_kana(ctx, &word2_as_word).await?.unwrap_or_default();
            format!("{}{}", k1, k2)
        }
    };
    // dict.lisp:639 — (or score-mod 0).
    // `(or score-mod 0)` evaluates to 0 only when score-mod is nil;
    // any truthy value (integer literal or `(constantly N)` closure)
    // passes through unchanged. The Rust port collapses `None` and
    // the integer-literal default to `ScoreMod::Single(0)`.
    let resolved_score_mod = score_mod.unwrap_or(ScoreMod::Single(0));

    match word1 {
        // dict.lisp:642-645 (defmethod adjoin-word ((word1 simple-text) (word2 simple-text)))
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Proxy(_) => {
            let word2_as_word = word2.to_word();
            // dict.lisp:644 — `:primary word1 :words (list word1 word2)`.
            // Lisp aliases the same word1 cell into both slots; the
            // Rust port clones into `primary` and moves the original
            // into `words`.
            let primary = Box::new(word1.clone());
            Ok(CompoundText {
                text: resolved_text,
                kana: resolved_kana,
                primary,
                words: vec![word1, word2_as_word],
                score_base: score_base.map(Box::new),
                score_mod: resolved_score_mod,
            })
        }
        // dict.lisp:647-652 (defmethod adjoin-word ((word1 compound-text) (word2 simple-text)))
        KaniWordDispatchEnum::Compound(mut compound) => {
            // dict.lisp:649 — setf text/kana on word1.
            compound.text = resolved_text;
            compound.kana = resolved_kana;
            // dict.lisp:650 — (append s-words (list word2)).
            compound.words.push(word2.to_word());
            // dict.lisp:651 — (funcall (if (listp s-score-mod) 'cons 'list)
            //                          score-mod s-score-mod).
            // `cons` prepends onto an existing list; `list` wraps two
            // non-list values into a fresh 2-list. Both branches end
            // with new value at index 0 and old at index 1+.
            compound.score_mod = match compound.score_mod {
                ScoreMod::Stack(stack) => {
                    let mut new_stack = Vec::with_capacity(stack.len() + 1);
                    new_stack.push(resolved_score_mod);
                    new_stack.extend(stack);
                    ScoreMod::Stack(new_stack)
                }
                other @ (ScoreMod::Single(_) | ScoreMod::Constant(_)) => {
                    ScoreMod::Stack(vec![resolved_score_mod, other])
                }
            };
            // dict.lisp:647 — `&allow-other-keys` drops :score-base;
            // word1's score-base slot is unchanged. The `score_base`
            // parameter is discarded in this branch.
            let _ = score_base;
            // dict.lisp:652 — `word1` is returned.
            Ok(compound)
        }
        // dict.lisp:632 — no method specialized on counter-text.
        // Upstream would signal `no-applicable-method`.
        KaniWordDispatchEnum::Counter(_) => {
            unreachable!(
                "adjoin-word has no method specialized on counter-text \
                 (upstream signals no-applicable-method)"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::simple_text_class::SimpleText;

    fn kanji(seq: i32, text: &str) -> KanjiText {
        KanjiText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        }
    }

    fn kana(seq: i32, text: &str) -> KanaText {
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

    // The `:around` text/kana defaults reach `get-kana` which hits
    // the database for kanji-text inputs; the unit tests below stay
    // synchronous by passing explicit `:text` / `:kana` keywords so
    // the `:around` defaulting paths fall through without touching
    // the DB. The text/kana defaulting paths are exercised in the
    // REPL probe `tests/repl_adjoin_concat_defaults` (see the
    // `repl_*` tests below) and pinned against the REPL transcript.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // ----- (simple-text, simple-text) primary method -----

    #[tokio::test]
    async fn simple_simple_explicit_text_and_kana() {
        // T2 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :text "abc" :kana "xyz" :score-mod 7)
        //   => COMPOUND-TEXT text="abc" kana="xyz" score-mod=7
        //                    score-base=NIL words=("食べ" "たい")
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("abc".into()),
            Some("xyz".into()),
            Some(ScoreMod::Single(7)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.text, "abc");
        assert_eq!(result.kana, "xyz");
        assert!(matches!(result.score_mod, ScoreMod::Single(7)));
        assert!(result.score_base.is_none());
        assert_eq!(result.words.len(), 2);
    }

    #[tokio::test]
    async fn simple_simple_primary_is_word1() {
        // T1 from REPL probe of upstream ichiran:
        //   words=("食べ" "たい"); primary is word1.
        // Pinned: result.primary derefs to the word1 input.
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            None,
            None,
        )
        .await
        .unwrap();
        match &*result.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10092273),
            _ => panic!("primary must be the input word1 (kanji-text)"),
        }
        // Words are [word1, word2].
        assert_eq!(result.words.len(), 2);
    }

    #[tokio::test]
    async fn simple_simple_score_mod_none_defaults_to_zero() {
        // T1 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2)  => score-mod=0
        // T8: explicit :score-mod nil => score-mod=0
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            None,
            None,
        )
        .await
        .unwrap();
        assert!(matches!(result.score_mod, ScoreMod::Single(0)));
    }

    #[tokio::test]
    async fn simple_simple_score_base_passthrough() {
        // T5 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :score-base w1)
        //   => score-base text="食べ"
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let sb = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(0)),
            Some(sb),
        )
        .await
        .unwrap();
        match result.score_base.as_deref() {
            Some(KaniWordDispatchEnum::Kanji(k)) => assert_eq!(k.text, "食べ"),
            _ => panic!("score-base must carry the kanji-text we passed in"),
        }
    }

    // ----- (compound-text, simple-text) primary method -----

    #[tokio::test]
    async fn compound_simple_appends_words_and_updates_text_kana() {
        // T3 from REPL probe of upstream ichiran:
        //   c3 = (adjoin-word w1 w2 :score-mod 3)  ; "食べたい" / "たべたい"
        //   c3b = (adjoin-word c3 w3 :score-mod 4) ; w3 = "ない"
        //   => c3b text="食べたいない" kana="たべたいない"
        //      words-text=("食べ" "たい" "ない")
        //      score-mod=(4 3)
        //      (eq c3 c3b)=T
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c3 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(3)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c3b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c3),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(4)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(c3b.text, "食べたいない");
        assert_eq!(c3b.kana, "たべたいない");
        assert_eq!(c3b.words.len(), 3);
        // dict.lisp:651 — first compound,simple adjoin: (list new old)
        match &c3b.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(4)));
                assert!(matches!(v[1], ScoreMod::Single(3)));
            }
            _ => panic!("score-mod must be Stack([4, 3]) after first compound adjoin"),
        }
    }

    #[tokio::test]
    async fn compound_simple_third_adjoin_grows_stack() {
        // T4 from REPL probe of upstream ichiran:
        //   c4  = (adjoin-word w1 w2 :score-mod 1)   ; single 1
        //   c4b = (adjoin-word c4 w3 :score-mod 2)   ; (2 1)
        //   c4c = (adjoin-word c4b w4 :score-mod 5)  ; (5 2 1)
        //   words-text=("食べ" "たい" "ない" "だ")
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c4 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c4b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c4),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            None,
        )
        .await
        .unwrap();
        let w4 = KaniSimpleTextDispatchEnum::Kana(kana(2089020, "だ"));
        let c4c = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c4b),
            w4,
            Some("食べたいないだ".into()),
            Some("たべたいないだ".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(c4c.words.len(), 4);
        match &c4c.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 3);
                assert!(matches!(v[0], ScoreMod::Single(5)));
                assert!(matches!(v[1], ScoreMod::Single(2)));
                assert!(matches!(v[2], ScoreMod::Single(1)));
            }
            _ => panic!("score-mod must be Stack([5, 2, 1]) after third adjoin"),
        }
    }

    #[tokio::test]
    async fn compound_simple_ignores_score_base() {
        // T6 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :score-mod 1 :score-base w1)  ; sets sb=w1
        //   (adjoin-word c6 w3 :score-mod 2 :score-base w2)  ; sb stays w1
        //   => after 2nd adjoin: score-base text="食べ"
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let sb_w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let c6 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            Some(sb_w1),
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        // Try to overwrite with :score-base w2 — should be ignored.
        let sb_w2 = KaniWordDispatchEnum::Kana(kana(1406940, "たい"));
        let c6b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c6),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            Some(sb_w2),
        )
        .await
        .unwrap();
        match c6b.score_base.as_deref() {
            Some(KaniWordDispatchEnum::Kanji(k)) => assert_eq!(k.text, "食べ"),
            _ => panic!("score-base must remain the originally-set w1 (Kanji '食べ')"),
        }
    }

    #[tokio::test]
    async fn compound_simple_primary_unchanged() {
        // T9 from REPL probe of upstream ichiran:
        //   After (adjoin-word c9 w3 ...) the primary slot's text is
        //   still "食べ" (the original word1 from the first adjoin),
        //   not w3's "ない".
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c9 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c9b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c9),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            None,
        )
        .await
        .unwrap();
        match &*c9b.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            _ => panic!("primary must remain the original word1 kanji-text"),
        }
    }

    // ----- :around default computation (hits DB via get-kana) -----

    #[tokio::test]
    async fn around_defaults_text_and_kana_to_concat() {
        // T1 from REPL probe of upstream ichiran:
        //   w1 = 食べ kanji-text seq 10092273 (get-kana "たべ" via best-kana-conj)
        //   w2 = たい kana-text  seq 1406940
        //   (adjoin-word w1 w2)
        //   => text="食べたい" kana="たべたい" score-mod=0
        //
        // Exercises the `:around` defaulting path: text concatenates
        // get-text outputs, kana concatenates get-kana outputs
        // (kanji-text's best-kana-conj hits the conjugation tables).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(&ctx, w1, w2, None, None, None, None).await.unwrap();
        assert_eq!(result.text, "食べたい");
        assert_eq!(result.kana, "たべたい");
        assert!(matches!(result.score_mod, ScoreMod::Single(0)));
    }

    // ----- ScoreMod::Constant — (constantly N) callsites -----

    #[tokio::test]
    async fn simple_simple_constant_score_mod() {
        // REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :text "ab" :kana "ab"
        //                      :score-mod (constantly 360))
        //   => slot = #<FUNCTION>  (functionp ✓, funcall returns 360
        //      regardless of argument).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        assert!(matches!(result.score_mod, ScoreMod::Constant(360)));
    }

    #[tokio::test]
    async fn compound_simple_grows_constant_into_list() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 360))
        //   c2 = (adjoin-word c1 w3 :score-mod 5)
        //   => c2 score-mod = (5 #<FUNCTION>) — Single(5) prepended
        //      onto the Constant(360).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        match &c2.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(5)));
                assert!(matches!(v[1], ScoreMod::Constant(360)));
            }
            _ => panic!("score-mod must be Stack([Single(5), Constant(360)])"),
        }
    }

    #[tokio::test]
    async fn compound_simple_constant_onto_constant() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 200))
        //   c2 = (adjoin-word c1 w3 :score-mod (constantly 300))
        //   => c2 score-mod = (#<300> #<200>) — Constant(300) prepended
        //      onto Constant(200).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(200)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Constant(300)),
            None,
        )
        .await
        .unwrap();
        match &c2.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Constant(300)));
                assert!(matches!(v[1], ScoreMod::Constant(200)));
            }
            _ => panic!("score-mod must be Stack([Constant(300), Constant(200)])"),
        }
    }

    #[tokio::test]
    async fn compound_simple_third_adjoin_mixes_constants_and_integers() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 360))
        //   c2 = (adjoin-word c1 w3 :score-mod 5)
        //   c3 = (adjoin-word c2 w4 :score-mod (constantly 200))
        //   => c3 score-mod = (#<200> 5 #<360>)
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        let w4 = KaniSimpleTextDispatchEnum::Kana(kana(2089020, "だ"));
        let c3 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c2),
            w4,
            Some("食べたいないだ".into()),
            Some("たべたいないだ".into()),
            Some(ScoreMod::Constant(200)),
            None,
        )
        .await
        .unwrap();
        match &c3.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 3);
                assert!(matches!(v[0], ScoreMod::Constant(200)));
                assert!(matches!(v[1], ScoreMod::Single(5)));
                assert!(matches!(v[2], ScoreMod::Constant(360)));
            }
            _ => panic!("score-mod must be Stack([Constant(200), Single(5), Constant(360)])"),
        }
    }
}
