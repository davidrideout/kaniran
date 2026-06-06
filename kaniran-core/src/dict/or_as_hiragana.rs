//! Port of `ichiran/dict:or-as-hiragana` (`dict-grammar.lisp:95`).
//!
//! Calls `fn_(word)`; if non-empty, returns those rows as
//! [`OrAsHiraganaRows::Direct`]. Otherwise re-enters through
//! [`find_word_as_hiragana`] with `fn_` as the finder, wrapping each
//! result as a [`ProxyText`] carrying `word` as both `text` and `kana`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word::FindWordRows;
use crate::dict::find_word_as_hiragana::{find_word_as_hiragana, HiraganaFinder};
use crate::dict::proxy_text_class::ProxyText;

/// Cloneable async closure called both directly and as the
/// `:finder` re-entry through [`find_word_as_hiragana`].
pub type OrAsHiraganaFinder<'a> = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<FindWordRows, sqlx::Error>> + Send + 'a>>
        + Send
        + Sync
        + 'a,
>;

#[derive(Debug, Clone)]
pub enum OrAsHiraganaRows {
    Direct(FindWordRows),
    AsHiragana(Vec<ProxyText>),
}

pub async fn or_as_hiragana<'a>(
    ctx: &'a KaniranContext,
    word: &str,
    fn_: OrAsHiraganaFinder<'a>,
) -> Result<Option<OrAsHiraganaRows>, sqlx::Error> {
    // dict-grammar.lisp:98 (let ((result (apply fn word args))) …)
    let result = fn_(word.to_string()).await?;
    let result_empty = match &result {
        FindWordRows::Kana(rows) => rows.is_empty(),
        FindWordRows::Kanji(rows) => rows.is_empty(),
    };
    if !result_empty {
        return Ok(Some(OrAsHiraganaRows::Direct(result)));
    }
    // dict-grammar.lisp:100 (find-word-as-hiragana word :finder (lambda (w) (apply fn w args)))
    let fn_clone = Arc::clone(&fn_);
    let finder: HiraganaFinder<'a> = Box::new(move |w| fn_clone(w));
    let proxies = find_word_as_hiragana(ctx, word, &[], Some(finder)).await?;
    if proxies.is_empty() {
        Ok(None)
    } else {
        Ok(Some(OrAsHiraganaRows::AsHiragana(proxies)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::find_word_with_pos::{find_word_with_pos, WordWithPosRows};
    use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

    // dict-grammar.lisp:506 (or-as-hiragana 'find-word-with-pos root …)
    fn make_pos_finder<'a>(
        ctx: &'a KaniranContext,
        posi: &'a [&'a str],
    ) -> OrAsHiraganaFinder<'a> {
        Arc::new(move |word: String| {
            Box::pin(async move {
                let rows = find_word_with_pos(ctx, &word, posi).await?;
                Ok(match rows {
                    WordWithPosRows::Kana(v) => FindWordRows::Kana(v),
                    WordWithPosRows::Kanji(v) => FindWordRows::Kanji(v),
                })
            })
        })
    }

    /// Path 1, kanji branch. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "私" "pn")` → 13
    /// KANJI-TEXT rows (same as
    /// `(find-word-with-pos "私" "pn")` because "私" has no kana-only
    /// variant to displace it).
    #[tokio::test]
    async fn kanji_direct_pn_thirteen_rows() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "私", finder).await.unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kanji = match direct {
            FindWordRows::Kanji(v) => v,
            FindWordRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 13);
        let mut got: Vec<(i32, i32)> = kanji.iter().map(|r| (r.seq, r.id)).collect();
        got.sort();
        let expected: Vec<(i32, i32)> = vec![
            (1311110, 22264),
            (1311125, 22265),
            (1347580, 26861),
            (2015370, 108229),
            (2079310, 114743),
            (2217330, 129111),
            (2217340, 129112),
            (2842390, 197077),
            (2845454, 199954),
            (2858221, 211749),
            (2858384, 211905),
            (2858397, 211916),
            (2864027, 217322),
        ];
        assert_eq!(got, expected);
    }

    /// Path 1, kana branch (katakana that has a direct katakana
    /// kana-text row → short-circuit, no fallback). REPL:
    /// `(or-as-hiragana 'find-word-with-pos "ジョギング" "vs")` →
    /// 1 KANA-TEXT row id=9654 seq=1066360.
    #[tokio::test]
    async fn katakana_direct_vs_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["vs"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "ジョギング", finder).await.unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kana = match direct {
            FindWordRows::Kana(v) => v,
            FindWordRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.id, 9654);
        assert_eq!(row.seq, 1066360);
        assert_eq!(row.text, "ジョギング");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(0));
        assert_eq!(row.common_tags, "[gai1][ichi1]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji, None);
    }

    /// Path 1, hiragana branch (pure hiragana → `as-hiragana` is
    /// identity → fallback can't fire; only the direct call can
    /// match). REPL: `(or-as-hiragana 'find-word-with-pos "わたし"
    /// "pn")` → 1 KANA-TEXT row id=38072 seq=1311110.
    #[tokio::test]
    async fn hiragana_direct_pn_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "わたし", finder).await.unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kana = match direct {
            FindWordRows::Kana(v) => v,
            FindWordRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.id, 38072);
        assert_eq!(row.seq, 1311110);
        assert_eq!(row.text, "わたし");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(1));
        assert_eq!(row.common_tags, "[ichi1][news1][nf01]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji.as_deref(), Some("私"));
    }

    /// Path 2a — katakana input with empty direct lookup but
    /// non-empty hiragana lookup. The fallback wraps each kana-text
    /// row in a proxy-text whose `text`/`kana` carry the original
    /// katakana surface form. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "アナタ" "pn")` →
    /// 2 PROXY-TEXT rows; both wrap kana-text rows for "あなた"
    /// (ids 29081 / 55771, seqs 1223615 / 1483180).
    #[tokio::test]
    async fn katakana_hiragana_fallback_two_proxies() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "アナタ", finder).await.unwrap();
        let proxies = match result {
            Some(OrAsHiraganaRows::AsHiragana(p)) => p,
            other => panic!("expected AsHiragana, got {:?}", other),
        };
        assert_eq!(proxies.len(), 2);
        for proxy in &proxies {
            assert_eq!(proxy.text, "アナタ");
            assert_eq!(proxy.kana, "アナタ");
        }
        let mut sources: Vec<(i32, i32, String)> = proxies
            .iter()
            .map(|p| match p.source.as_ref() {
                KaniSimpleTextDispatchEnum::Kana(row) => (row.seq, row.id, row.text.clone()),
                KaniSimpleTextDispatchEnum::Kanji(row) => (row.seq, row.id, row.text.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => {
                    panic!("REPL pinned source to KANA-TEXT; got nested PROXY-TEXT")
                }
            })
            .collect();
        sources.sort();
        assert_eq!(
            sources,
            vec![
                (1223615, 29081, "あなた".to_string()),
                (1483180, 55771, "あなた".to_string()),
            ]
        );
    }

    /// Path None — both direct and hiragana lookup empty. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "コノ" "pn")` → NIL
    /// (no kana-text or kanji-text rows for either katakana
    /// "コノ" or its hiragana form "この" with the "pn" pos tag).
    #[tokio::test]
    async fn katakana_both_empty_yields_none() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "コノ", finder).await.unwrap();
        assert!(result.is_none());
    }

    /// Path None — kanji input with no pos match; `as-hiragana`
    /// leaves kanji intact, so the fallback path also produces
    /// nothing (the str/as-hiragana equality short-circuit inside
    /// `find_word_as_hiragana` returns an empty Vec). REPL:
    /// `(or-as-hiragana 'find-word-with-pos "青空" "vs")` → NIL.
    #[tokio::test]
    async fn kanji_no_match_yields_none() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["vs"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "青空", finder).await.unwrap();
        assert!(result.is_none());
    }
}
