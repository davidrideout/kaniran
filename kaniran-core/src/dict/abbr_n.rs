//! Port of `ichiran/dict:abbr-n` (`dict-grammar.lisp:602-608`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-n :nai-n 2 (root)
//!   (find-word-with-conj-prop
//!    (concatenate 'string root "ない")
//!    (lambda (cdata)
//!      ;; 居ない 来ない create problems so they are blocked
//!      (and (not (find (conj-data-from cdata) '(1577980 1547720)))
//!           (conj-neg (conj-data-prop cdata))))))
//! ```
//!
//! Same body as `abbr-nee` modulo `:allow-root` (this callsite omits
//! it, defaulting to nil). Mapcar tail delegated to
//! [`def_abbr_suffix_body`] (CONVENTIONS §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn abbr_n(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "ない");
    let primary_words = find_word_with_conj_prop(
        &ctx_rebound,
        &wordstr,
        // dict-grammar.lisp:605-607 — (and (not (find (conj-data-from cdata)
        //                                            '(1577980 1547720)))
        //                                  (conj-neg (conj-data-prop cdata)))
        |cdata| {
            let from_blocked = cdata
                .from
                .is_some_and(|f| f == 1577980 || f == 1547720);
            !from_blocked && cdata.prop.as_ref().is_some_and(|p| p.neg != Some(false))
        },
        false,
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

    /// REPL N1: `(abbr-n "知ら" "ん" nil)` → 1 PROXY text="知らん"
    /// kana="しらん" hintedp=T source=KANJI-TEXT (seq 10105960,
    /// text "知らない").
    #[tokio::test]
    async fn n1_shira_n_kanji() {
        let ctx = ctx().await;
        let result = abbr_n(&ctx, "知ら", "ん", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "知らん");
        assert_eq!(p.kana, "しらん");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10105960);
    }

    /// REPL N2: `(abbr-n "食べ" "ん" nil)` → 1 PROXY text="食べん"
    /// kana="たべん" source=KANJI-TEXT (seq 10092227).
    #[tokio::test]
    async fn n2_tabe_n_kanji() {
        let ctx = ctx().await;
        let result = abbr_n(&ctx, "食べ", "ん", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "食べん");
        assert_eq!(p.kana, "たべん");
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10092227);
    }

    /// REPL N3: `(abbr-n "い" "ん" nil)` → 5 PROXY (kana-source
    /// branch + no-allow_root behavior). 居ない (from=1577980)
    /// blocked; entry 1155180 is "いない" with empty conj-data
    /// which `abbr-nee`'s `:allow-root t` would have collected but
    /// `abbr-n` omits — so we observe 5 here vs 6 for abbr-nee.
    /// Source-seqs sorted: {10033628, 10128866, 10303114, 10362292,
    /// 10423265}.
    #[tokio::test]
    async fn n3_i_kana_source_no_allow_root() {
        let ctx = ctx().await;
        let result = abbr_n(&ctx, "い", "ん", None).await.unwrap();
        assert_eq!(result.len(), 5);
        for w in &result {
            let KaniWordDispatchEnum::Proxy(p) = w else {
                panic!("expected Proxy");
            };
            assert_eq!(p.text, "いん");
            assert_eq!(p.kana, "いん");
            assert!(p.state.hintedp);
            // dict.lisp:70 — (conjugations :initform nil) → None on the
            // proxy's own slot (the word-conjugations accessor reads
            // through to source via dict.lisp:568-569; this assertion
            // pins the proxy's own slot default, not the accessor result).
            assert_eq!(p.state.conjugations, None);
            let KaniSimpleTextDispatchEnum::Kana(k) = &*p.source else {
                panic!("expected Kana source");
            };
            assert_eq!(k.text, "いない");
        }
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
        assert_eq!(seqs, vec![10033628, 10128866, 10303114, 10362292, 10423265]);
        // Root entry 1155180 (collectable only via allow_root) MUST NOT
        // appear in abbr-n's output.
        assert!(!seqs.contains(&1155180));
    }
}
