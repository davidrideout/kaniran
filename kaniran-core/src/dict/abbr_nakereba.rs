//! Port of `ichiran/dict:abbr-nakereba` (`dict-grammar.lisp:612-613`).
//!
//! Matches the spoken abbreviation of `root + "なければ"` (e.g.
//! 行かなきゃ for 行かなければ).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn abbr_nakereba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "なければ");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 4, None).await
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

    /// REPL NAKEREBA1: `(abbr-nakereba "行か" "なきゃ" nil)` → 1 PROXY
    /// text="行かなきゃ" kana="いかなきゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10349404, text "行かなければ").
    #[tokio::test]
    async fn nakereba1_ika_nakya() {
        let ctx = ctx().await;
        let result = abbr_nakereba(&ctx, "行か", "なきゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "行かなきゃ");
        assert_eq!(p.kana, "いかなきゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10349404);
    }

    /// REPL NAKEREBA2: `(abbr-nakereba "食べ" "なくちゃ" nil)` → 1 PROXY
    /// text="食べなくちゃ" kana="たべなくちゃ" source=KANJI-TEXT
    /// (seq 10092239, text "食べなければ").
    #[tokio::test]
    async fn nakereba2_tabe_nakucha() {
        let ctx = ctx().await;
        let result = abbr_nakereba(&ctx, "食べ", "なくちゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "食べなくちゃ");
        assert_eq!(p.kana, "たべなくちゃ");
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10092239);
    }

    /// REPL NAKEREBA3: `(abbr-nakereba "やら" "ねば" nil)` → 4 PROXY
    /// text="やらねば" kana="やらねば" source=KANA-TEXT sorted seqs
    /// {10038002, 10366027, 10402893, 10463965}. Polysemy.
    #[tokio::test]
    async fn nakereba3_yara_neba_polysemy() {
        let ctx = ctx().await;
        let result = abbr_nakereba(&ctx, "やら", "ねば", None).await.unwrap();
        assert_eq!(result.len(), 4);
        for w in &result {
            let KaniWordDispatchEnum::Proxy(p) = w else {
                panic!("expected Proxy");
            };
            assert_eq!(p.text, "やらねば");
            assert_eq!(p.kana, "やらねば");
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
        assert_eq!(seqs, vec![10038002, 10366027, 10402893, 10463965]);
    }
}
