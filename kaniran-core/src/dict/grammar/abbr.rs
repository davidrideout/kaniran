//! Port of the dict-grammar.lisp `def-abbr-suffix` macro + 15 callsites.

use crate::characters::char_classes::CharClass;
use crate::characters::text_utils::destem;
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_text::get_kana;
use crate::dict::dao::KanaText;
use crate::dict::find_word::find_word_full;
use crate::dict::grammar::find_word::{find_word_conj_of, find_word_with_conj_prop, WordSeqRows};
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::text_classes::{ProxyText, SimpleText};

pub async fn def_abbr_suffix_body(
    ctx: &KaniranContext,
    primary_words: Vec<KaniWordDispatchEnum>,
    root: &str,
    suf_var: &str,
    stem: usize,
    patch: Option<(&str, &str)>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let mut out = Vec::with_capacity(primary_words.len());
    for pw in primary_words {
        // dict-grammar.lisp:559 — (concatenate 'string root-var suf-var)
        let text = format!("{}{}", root, suf_var);
        // dict-grammar.lisp:560 — (let ((k (get-kana pw))) …); nil → ""
        let pw_kana = get_kana(ctx, &pw).await?.unwrap_or_default();
        // dict-grammar.lisp:561-566 — (if patch-var (destem k (length car) + cdr) (destem k stem))
        let pw_kana_trimmed = match patch {
            Some((car, cdr)) => {
                let car_len = car.chars().count();
                format!("{}{}", destem(&pw_kana, car_len, CharClass::Kana), cdr)
            }
            None => destem(&pw_kana, stem, CharClass::Kana),
        };
        // dict-grammar.lisp:567 — (concatenate 'string <pw_kana_trimmed> suf-var)
        let kana = format!("{}{}", pw_kana_trimmed, suf_var);
        // dict-grammar.lisp:568-578 — (etypecase pw (simple-text …) (compound-text …))
        match pw {
            // dict-grammar.lisp:569-574 (simple-text arm) —
            // (make-instance 'proxy-text :source pw :text :kana :hintedp t)
            KaniWordDispatchEnum::Kanji(k) => {
                out.push(KaniWordDispatchEnum::Proxy(ProxyText {
                    text,
                    kana,
                    source: Box::new(KaniSimpleTextDispatchEnum::Kanji(k)),
                    state: SimpleText {
                        conjugations: None,
                        hintedp: true,
                    },
                }));
            }
            KaniWordDispatchEnum::Kana(k) => {
                out.push(KaniWordDispatchEnum::Proxy(ProxyText {
                    text,
                    kana,
                    source: Box::new(KaniSimpleTextDispatchEnum::Kana(k)),
                    state: SimpleText {
                        conjugations: None,
                        hintedp: true,
                    },
                }));
            }
            KaniWordDispatchEnum::Proxy(p) => {
                out.push(KaniWordDispatchEnum::Proxy(ProxyText {
                    text,
                    kana,
                    source: Box::new(KaniSimpleTextDispatchEnum::Proxy(p)),
                    state: SimpleText {
                        conjugations: None,
                        hintedp: true,
                    },
                }));
            }
            // dict-grammar.lisp:575-578 (compound-text arm) —
            // (with-slots ((stext text) (skana kana)) pw (setf stext text skana kana)) pw
            KaniWordDispatchEnum::Compound(mut c) => {
                c.text = text;
                c.kana = kana;
                out.push(KaniWordDispatchEnum::Compound(c));
            }
            // dict-grammar.lisp:568 (etypecase) — counter-text is
            // not a `simple-text` or `compound-text`; upstream raises
            // a TYPE-ERROR. No upstream callsite reaches here (abbr
            // primary-words sources are find-word-full / find-word-
            // with-conj-prop / find-word-conj-of with no :counter).
            KaniWordDispatchEnum::Counter(_) => {
                panic!("def-abbr-suffix etypecase received counter-text")
            }
        }
    }
    Ok(out)
}

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
            let from_blocked = cdata.from.is_some_and(|f| f == 1577980 || f == 1547720);
            !from_blocked && cdata.prop.as_ref().is_some_and(|p| p.neg != Some(false))
        },
        true,
    )
    .await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_nx(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict-grammar.lisp:555 — (let* ((*suffix-map-temp* nil) …))
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:592-600 (cond ((equal root "せ") …) (t …))
    let (primary_words, patch): (Vec<KaniWordDispatchEnum>, Option<(&str, &str)>) = if root == "せ"
    {
        // dict-grammar.lisp:593-594 — (setf patch '("しない" . "せ"))
        //                              (find-word-conj-of "しない" 1157170)
        let rows = find_word_conj_of(&ctx_rebound, "しない", &[1157170]).await?;
        let words: Vec<KaniWordDispatchEnum> = match rows {
            WordSeqRows::Kana(v) => v.into_iter().map(KaniWordDispatchEnum::Kana).collect(),
            WordSeqRows::Kanji(v) => v.into_iter().map(KaniWordDispatchEnum::Kanji).collect(),
        };
        (words, Some(("しない", "せ")))
    } else {
        // dict-grammar.lisp:596-600 — (find-word-with-conj-prop (concatenate root "ない") λ)
        let wordstr = format!("{}{}", root, "ない");
        let words = find_word_with_conj_prop(
            &ctx_rebound,
            &wordstr,
            // dict-grammar.lisp:598-599 — (and (/= (conj-data-from cdata) 1157170)
            //                                   (conj-neg (conj-data-prop cdata)))
            |cdata| {
                cdata.from != Some(1157170)
                    && cdata.prop.as_ref().is_some_and(|p| p.neg != Some(false))
            },
            false,
        )
        .await?;
        (words, None)
    };

    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, patch).await
}

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
            let from_blocked = cdata.from.is_some_and(|f| f == 1577980 || f == 1547720);
            !from_blocked && cdata.prop.as_ref().is_some_and(|p| p.neg != Some(false))
        },
        false,
    )
    .await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

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

pub async fn abbr_shimasho(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "しましょう");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 5, None).await
}

pub async fn abbr_dewanai(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "ではない");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 4, None).await
}

pub async fn abbr_teba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "てば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_reba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "れば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_keba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "けば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_geba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "げば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_neba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "ねば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_beba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "べば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_meba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "めば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_seba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "せば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

pub async fn abbr_ii(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "いい");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

#[cfg(test)]
mod abbr_nee_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

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

#[cfg(test)]
mod abbr_nx_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL NX1: `(abbr-nx "知ら" "ず" nil)` → 1 PROXY
    /// text="知らず" kana="しらず" hintedp=T source=KANJI-TEXT
    /// (seq 10105960, text "知らない"). Exercises the t-arm filter.
    #[tokio::test]
    async fn nx1_shira_zu_kanji() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "知ら", "ず", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "知らず");
        assert_eq!(p.kana, "しらず");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10105960);
    }

    /// REPL NX2: `(abbr-nx "食べ" "ず" nil)` → 1 PROXY
    /// text="食べず" kana="たべず" source=KANJI-TEXT seq=10092227.
    #[tokio::test]
    async fn nx2_tabe_zu_kanji() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "食べ", "ず", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "食べず");
        assert_eq!(p.kana, "たべず");
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10092227);
    }

    /// REPL NX3: `(abbr-nx "せ" "ず" nil)` → 1 PROXY text="せず"
    /// kana="せず" source=KANA-TEXT (seq=10152244, text "しない").
    /// Exercises the patch branch: `destem("しない", 3) = ""` +
    /// patch-cdr "せ" + suf-var "ず" → "せず".
    #[tokio::test]
    async fn nx3_se_zu_patch_branch() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "せ", "ず", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "せず");
        assert_eq!(p.kana, "せず");
        assert!(p.state.hintedp);
        assert_eq!(p.state.conjugations, None);
        let KaniSimpleTextDispatchEnum::Kana(k) = &*p.source else {
            panic!("expected Kana source");
        };
        assert_eq!(k.seq, 10152244);
        assert_eq!(k.text, "しない");
    }

    /// REPL NX4: `(abbr-nx "せ" "ざる" nil)` → 1 PROXY text="せざる"
    /// kana="せざる" (patch branch with different suf-var).
    #[tokio::test]
    async fn nx4_se_zaru_patch_branch() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "せ", "ざる", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "せざる");
        assert_eq!(p.kana, "せざる");
    }

    /// REPL NX5: `(abbr-nx "せ" "ぬ" nil)` → 1 PROXY text="せぬ"
    /// kana="せぬ" (patch branch with yet another suf-var).
    #[tokio::test]
    async fn nx5_se_nu_patch_branch() {
        let ctx = ctx().await;
        let result = abbr_nx(&ctx, "せ", "ぬ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "せぬ");
        assert_eq!(p.kana, "せぬ");
    }
}

#[cfg(test)]
mod abbr_n_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

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

#[cfg(test)]
mod abbr_nakereba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

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

#[cfg(test)]
mod abbr_shimasho_tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL SHIMASHO1: `(abbr-shimasho "勉強" "しましょ" nil)` → 1
    /// COMPOUND text="勉強しましょ" kana="べんきょう しましょ".
    /// Exercises the compound-text branch of the etypecase: input
    /// is `find-word-full "勉強しましょう"` → compound from
    /// `suffix-suru` with text="勉強しましょう", kana=
    /// "べんきょう しましょう"; the abbr body mutates text/kana
    /// in-place to the abbreviated forms. primary, words, score_mod,
    /// score_base on the compound are NOT touched
    /// (dict-grammar.lisp:575-578 only `setf`s `stext` and `skana`).
    #[tokio::test]
    async fn shimasho1_benkyou_shimasho_compound_branch() {
        use crate::dict::kani::KaniWordDispatchEnum as W;
        let ctx = ctx().await;
        let result = abbr_shimasho(&ctx, "勉強", "しましょ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let W::Compound(c) = &result[0] else {
            panic!("expected Compound");
        };
        assert_eq!(c.text, "勉強しましょ");
        assert_eq!(c.kana, "べんきょう しましょ");
        // primary unchanged from the suffix-suru compound.
        let W::Kanji(primary) = &*c.primary else {
            panic!("expected Kanji primary");
        };
        assert_eq!(primary.seq, 1512670);
        assert_eq!(primary.text, "勉強");
        // words unchanged: [KANJI-TEXT 勉強, KANA-TEXT しましょう].
        assert_eq!(c.words.len(), 2);
        let W::Kanji(w0) = &c.words[0] else {
            panic!("expected words[0] Kanji");
        };
        assert_eq!(w0.seq, 1512670);
        assert_eq!(w0.text, "勉強");
        let W::Kana(w1) = &c.words[1] else {
            panic!("expected words[1] Kana");
        };
        assert_eq!(w1.seq, 10152277);
        assert_eq!(w1.text, "しましょう");
    }
}

#[cfg(test)]
mod abbr_dewanai_tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL DEWANAI1: `(abbr-dewanai "犬" "じゃない" nil)` → NIL.
    /// `find-word-full "犬ではない"` returns no rows (it's a phrase,
    /// not an entry / conjugated form), so primary-words is empty
    /// and mapcar yields nil.
    #[tokio::test]
    async fn dewanai1_inu_dehanai_empty() {
        let ctx = ctx().await;
        let result = abbr_dewanai(&ctx, "犬", "じゃない", None).await.unwrap();
        assert!(result.is_empty());
    }
}

#[cfg(test)]
mod abbr_teba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL TEBA1: `(abbr-teba "立" "ちゃ" nil)` → 1 PROXY
    /// text="立ちゃ" kana="たちゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10435976, text "立てば").
    #[tokio::test]
    async fn teba1_tatsu_cha() {
        let ctx = ctx().await;
        let result = abbr_teba(&ctx, "立", "ちゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "立ちゃ");
        assert_eq!(p.kana, "たちゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10435976);
    }

    /// REPL TEBA2: `(abbr-teba "勝" "ちゃ" nil)` → 1 PROXY
    /// text="勝ちゃ" kana="かちゃ" source=KANJI-TEXT (seq 10316061).
    #[tokio::test]
    async fn teba2_katsu_cha() {
        let ctx = ctx().await;
        let result = abbr_teba(&ctx, "勝", "ちゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "勝ちゃ");
        assert_eq!(p.kana, "かちゃ");
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10316061);
    }
}

#[cfg(test)]
mod abbr_reba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL REBA1: `(abbr-reba "見" "りゃ" nil)` → 1 PROXY
    /// text="見りゃ" kana="みりゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10315017, text "見れば").
    #[tokio::test]
    async fn reba1_miru_rya() {
        let ctx = ctx().await;
        let result = abbr_reba(&ctx, "見", "りゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "見りゃ");
        assert_eq!(p.kana, "みりゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10315017);
    }
}

#[cfg(test)]
mod abbr_keba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL KEBA1: `(abbr-keba "書" "きゃ" nil)` → 1 PROXY
    /// text="書きゃ" kana="かきゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10526936, text "書けば").
    #[tokio::test]
    async fn keba1_kaku_kya() {
        let ctx = ctx().await;
        let result = abbr_keba(&ctx, "書", "きゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "書きゃ");
        assert_eq!(p.kana, "かきゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10526936);
    }
}

#[cfg(test)]
mod abbr_geba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL GEBA1: `(abbr-geba "泳" "ぎゃ" nil)` → 1 PROXY
    /// text="泳ぎゃ" kana="およぎゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10485536, text "泳げば").
    #[tokio::test]
    async fn geba1_oyogu_gya() {
        let ctx = ctx().await;
        let result = abbr_geba(&ctx, "泳", "ぎゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "泳ぎゃ");
        assert_eq!(p.kana, "およぎゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10485536);
    }
}

#[cfg(test)]
mod abbr_neba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL NEBA1: `(abbr-neba "死" "にゃ" nil)` → 1 PROXY
    /// text="死にゃ" kana="しにゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10236417, text "死ねば").
    #[tokio::test]
    async fn neba1_shinu_nya() {
        let ctx = ctx().await;
        let result = abbr_neba(&ctx, "死", "にゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "死にゃ");
        assert_eq!(p.kana, "しにゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10236417);
    }
}

#[cfg(test)]
mod abbr_beba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL BEBA1: `(abbr-beba "遊" "びゃ" nil)` → 2 PROXY
    /// text="遊びゃ" source=KANJI-TEXT seqs {10202469, 10225128}.
    /// 遊 has two readings → polysemy: 10202469 (kana="すさぶ") and
    /// 10225128 (kana="あそぶ"), both source-text="遊べば" but
    /// distinct source-kanas → distinct proxy kanas
    /// "すさびゃ" / "あそびゃ".
    #[tokio::test]
    async fn beba1_asobu_polysemy() {
        let ctx = ctx().await;
        let result = abbr_beba(&ctx, "遊", "びゃ", None).await.unwrap();
        assert_eq!(result.len(), 2);
        for w in &result {
            let KaniWordDispatchEnum::Proxy(p) = w else {
                panic!("expected Proxy");
            };
            assert_eq!(p.text, "遊びゃ");
            assert!(p.state.hintedp);
            let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
                panic!("expected Kanji source");
            };
            assert_eq!(k.text, "遊べば");
        }
        // Two reading branches: すさぶ → "すさびゃ" and あそぶ → "あそびゃ".
        let mut kanas: Vec<String> = result
            .iter()
            .map(|w| match w {
                KaniWordDispatchEnum::Proxy(p) => p.kana.clone(),
                _ => unreachable!(),
            })
            .collect();
        kanas.sort();
        assert_eq!(kanas, vec!["あそびゃ", "すさびゃ"]);
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
        assert_eq!(seqs, vec![10202469, 10225128]);
    }
}

#[cfg(test)]
mod abbr_meba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL MEBA1: `(abbr-meba "飲" "みゃ" nil)` → 1 PROXY
    /// text="飲みゃ" kana="のみゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10665831, text "飲めば").
    #[tokio::test]
    async fn meba1_nomu_mya() {
        let ctx = ctx().await;
        let result = abbr_meba(&ctx, "飲", "みゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "飲みゃ");
        assert_eq!(p.kana, "のみゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10665831);
    }
}

#[cfg(test)]
mod abbr_seba_tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL SEBA1: `(abbr-seba "話" "しゃ" nil)` → 1 PROXY
    /// text="話しゃ" kana="はなしゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10143263, text "話せば").
    #[tokio::test]
    async fn seba1_hanasu_sha() {
        let ctx = ctx().await;
        let result = abbr_seba(&ctx, "話", "しゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "話しゃ");
        assert_eq!(p.kana, "はなしゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10143263);
    }
}

#[cfg(test)]
mod abbr_ii_tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL II1: `(abbr-ii "良" "ええ" nil)` → NIL.
    /// `find-word-full "良いい"` returns no rows.
    #[tokio::test]
    async fn ii1_yoi_empty() {
        let ctx = ctx().await;
        let result = abbr_ii(&ctx, "良", "ええ", None).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL II2: `(abbr-ii "い" "ええ" nil)` → NIL.
    /// `find-word-full "いいい"` returns no rows.
    #[tokio::test]
    async fn ii2_i_empty() {
        let ctx = ctx().await;
        let result = abbr_ii(&ctx, "い", "ええ", None).await.unwrap();
        assert!(result.is_empty());
    }
}
