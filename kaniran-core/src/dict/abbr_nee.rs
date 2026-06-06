//! Port of `ichiran/dict:abbr-nee` (`dict-grammar.lisp:582-589`).
//!
//! Matches the negative `root + "ない"` (allowing the bare root),
//! blocking 居ない and 来ない which cause problems.

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn abbr_nee(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict-grammar.lisp:555 — (let* ((*suffix-map-temp* nil) …))
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    // dict-grammar.lisp:583-589 (find-word-with-conj-prop (concatenate root "ない") λ :allow-root t)
    let wordstr = format!("{}{}", root, "ない");
    let primary_words = find_word_with_conj_prop(
        &ctx_rebound,
        &wordstr,
        // dict-grammar.lisp:585-588 — (lambda (cdata)
        //   (and (not (find (conj-data-from cdata) '(1577980 1547720)))
        //        (conj-neg (conj-data-prop cdata))))
        |cdata| {
            let from_blocked = cdata
                .from
                .is_some_and(|f| f == 1577980 || f == 1547720);
            !from_blocked && cdata.prop.as_ref().is_some_and(|p| p.neg != Some(false))
        },
        true,
    )
    .await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL NEE1: `(abbr-nee "食べ" "ねえ" nil)` → 1 PROXY
    /// text="食べねえ" kana="たべねえ" hintedp=T source=KANJI-TEXT
    /// (seq 10092227, text "食べない").
    #[tokio::test]
    async fn nee1_taberu_kanji() {
        let ctx = ctx().await;
        let result = abbr_nee(&ctx, "食べ", "ねえ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "食べねえ");
        assert_eq!(p.kana, "たべねえ");
        assert!(p.state.hintedp);
        // dict.lisp:70 — (conjugations :initform nil); make-instance
        // proxy-text at dict-grammar.lisp:569-574 omits :conjugations
        // so the proxy's own slot defaults to nil.
        assert_eq!(p.state.conjugations, None);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10092227);
        assert_eq!(k.text, "食べない");
    }

    /// REPL NEE2: `(abbr-nee "知ら" "ねえ" nil)` → 2 PROXY
    /// text="知らねえ" kana="しらねえ" hintedp=T source-seqs sorted
    /// {1420420, 10105960}.
    #[tokio::test]
    async fn nee2_shiraneru_polysemy() {
        let ctx = ctx().await;
        let result = abbr_nee(&ctx, "知ら", "ねえ", None).await.unwrap();
        assert_eq!(result.len(), 2);
        for w in &result {
            let KaniWordDispatchEnum::Proxy(p) = w else {
                panic!("expected Proxy");
            };
            assert_eq!(p.text, "知らねえ");
            assert_eq!(p.kana, "しらねえ");
            assert!(p.state.hintedp);
            let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
                panic!("expected Kanji source");
            };
            assert_eq!(k.text, "知らない");
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|w| match w {
                KaniWordDispatchEnum::Proxy(p) => match &*p.source {
                    KaniSimpleTextDispatchEnum::Kanji(k) => k.seq,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![1420420, 10105960]);
    }

    /// REPL NEE3: `(abbr-nee "い" "ねえ" nil)` → 6 PROXY. 居ない
    /// (seq 1577980) is blocked, but other "いない" entries (10033628,
    /// 10128866, 10303114, 10362292, 10423265, 1155180) pass the
    /// filter — exercises the from-not-in-blocklist branch.
    #[tokio::test]
    async fn nee3_i_blocks_iru_passes_others() {
        let ctx = ctx().await;
        let result = abbr_nee(&ctx, "い", "ねえ", None).await.unwrap();
        assert_eq!(result.len(), 6);
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|w| match w {
                KaniWordDispatchEnum::Proxy(p) => match &*p.source {
                    KaniSimpleTextDispatchEnum::Kana(k) => k.seq,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(
            seqs,
            vec![1155180, 10033628, 10128866, 10303114, 10362292, 10423265]
        );
        // 居ない (seq 1577980) must NOT appear.
        assert!(!seqs.contains(&1577980));
    }

    /// REPL NEE4: `(abbr-nee "こ" "ねえ" nil)` → 1 PROXY source=
    /// KANA-TEXT seq=2398700. 来ない (from=1547720) is blocked but
    /// "こない" entry 2398700 derives from a different from and
    /// passes.
    #[tokio::test]
    async fn nee4_ko_blocks_kuru_passes_konai() {
        let ctx = ctx().await;
        let result = abbr_nee(&ctx, "こ", "ねえ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        let KaniSimpleTextDispatchEnum::Kana(k) = &*p.source else {
            panic!("expected Kana source");
        };
        assert_eq!(k.seq, 2398700);
    }
}
