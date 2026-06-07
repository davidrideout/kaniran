use crate::characters::char_class::CharClass;
use crate::characters::kana::destem;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_full::find_word_full;
use crate::dict::get_kana::get_kana;
use crate::dict::grammar::lookup::{find_word_conj_of, find_word_with_conj_prop, WordSeqRows};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::simple_text_class::SimpleText;

/// Port of `ichiran/dict:def-abbr-suffix` (`dict-grammar.lisp:547-579`).
///
/// Shared body for the abbreviated-suffix definers: maps each
/// primary word to a proxy-text (or in-place-rewritten compound-text)
/// whose text/kana splice the root and suffix, destemming the kana by
/// `stem` (or by the optional `patch-var` prefix).
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

/// Port of `ichiran/dict:abbr-nee` (`dict-grammar.lisp:582-589`).
///
/// Matches the negative `root + "ない"` (allowing the bare root),
/// blocking 居ない and 来ない which cause problems.
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

/// Port of `ichiran/dict:abbr-nx` (`dict-grammar.lisp:591-600`).
///
/// Matches the negative `root + "ない"`, with a special-cased patch
/// mapping せ to する's しない form.
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

/// Port of `ichiran/dict:abbr-n` (`dict-grammar.lisp:602-608`).
///
/// Matches the contracted negative `root + "ない"` (the ん form),
/// blocking 居ない and 来ない which cause problems.
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

/// Port of `ichiran/dict:abbr-nakereba` (`dict-grammar.lisp:612-613`).
///
/// Matches the spoken abbreviation of `root + "なければ"` (e.g.
/// 行かなきゃ for 行かなければ).
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

/// Port of `ichiran/dict:abbr-shimasho` (`dict-grammar.lisp:615-616`).
///
/// Matches the spoken abbreviation of `root + "しましょう"` (e.g.
/// 勉強しましょ for 勉強しましょう).
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

/// Port of `ichiran/dict:abbr-dewanai` (`dict-grammar.lisp:618-619`).
///
/// `:dewanai` abbreviated suffix: looks up `root + "ではない"` and
/// produces the suffix candidates for it.
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

/// Port of `ichiran/dict:abbr-teba` (`dict-grammar.lisp:626-627`).
///
/// Matches the spoken abbreviation of `root + "てば"` (e.g. 立ちゃ
/// for 立てば).
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

/// Port of `ichiran/dict:abbr-reba` (`dict-grammar.lisp:629-630`).
///
/// Matches the spoken abbreviation of `root + "れば"` (e.g. 見りゃ
/// for 見れば).
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

/// Port of `ichiran/dict:abbr-keba` (`dict-grammar.lisp:632-633`).
///
/// Matches the spoken abbreviation of `root + "けば"` (e.g. 書きゃ
/// for 書けば).
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

/// Port of `ichiran/dict:abbr-geba` (`dict-grammar.lisp:635-636`).
///
/// `:geba` abbreviated suffix: looks up `root + "げば"` and produces
/// the suffix candidates for it.
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

/// Port of `ichiran/dict:abbr-neba` (`dict-grammar.lisp:638-639`).
///
/// Matches the spoken abbreviation of `root + "ねば"` (e.g. 死にゃ
/// for 死ねば).
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

/// Port of `ichiran/dict:abbr-beba` (`dict-grammar.lisp:641-642`).
///
/// `:beba` abbreviated suffix: looks up `root + "べば"` and produces
/// the suffix candidates for it.
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

/// Port of `ichiran/dict:abbr-meba` (`dict-grammar.lisp:644-645`).
///
/// Matches the spoken abbreviation of `root + "めば"` (e.g. 飲みゃ
/// for 飲めば).
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

/// Port of `ichiran/dict:abbr-seba` (`dict-grammar.lisp:647-648`).
///
/// Matches the spoken abbreviation of `root + "せば"` (e.g. 話しゃ
/// for 話せば).
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

/// Port of `ichiran/dict:abbr-ii` (`dict-grammar.lisp:660-661`).
///
/// `:ii` abbreviated suffix: looks up `root + "いい"` and produces the
/// suffix candidates for it.
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
mod tests;
