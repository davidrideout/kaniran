//! Port of `ichiran/dict:match-sense-restrictions` (`dict.lisp:1515`).
//!
//! Tests whether a sense (given its `stagk`/`stagr` restriction tags in
//! `props`) applies to `reading`: true when there are no restrictions or
//! the reading is listed, false when the wrong word-type is restricted
//! away, otherwise a `restricted-readings` lookup paired with
//! [`super::match_kana_kanji`].

use crate::conn::kani_context::KaniranContext;

use super::kana_text_dao::KanaText;
use super::kani_word::KaniWordDispatchEnum;
use super::kanji_text_dao::KanjiText;
use super::match_kana_kanji::{match_kana_kanji, MatchKanaKanjiResult};
use super::text::text;
use super::word_type::{word_type, WordType};

pub async fn match_sense_restrictions(
    ctx: &KaniranContext,
    seq: i32,
    props: &[(String, Vec<String>)],
    reading: &KaniWordDispatchEnum,
) -> Result<Option<MatchKanaKanjiResult>, sqlx::Error> {
    // dict.lisp:1516-1517 (stagk/stagr (cdr (assoc … props :test 'equal)))
    let stagk: &[String] = props
        .iter()
        .find(|(tag, _)| tag == "stagk")
        .map_or(&[], |(_, texts)| texts.as_slice());
    let stagr: &[String] = props
        .iter()
        .find(|(tag, _)| tag == "stagr")
        .map_or(&[], |(_, texts)| texts.as_slice());
    // dict.lisp:1518 (wtype (word-type reading))
    let wtype = word_type(reading);

    // dict.lisp:1519 ((and (not stagk) (not stagr)) t)
    if stagk.is_empty() && stagr.is_empty() {
        return Ok(Some(MatchKanaKanjiResult::Yes));
    }
    // dict.lisp:1520-1521 ((or (member (text reading) stagk) (member (text reading) stagr)) t)
    let reading_text = text(reading);
    let reading_text = reading_text.as_ref();
    if stagk.iter().any(|t| t.as_str() == reading_text)
        || stagr.iter().any(|t| t.as_str() == reading_text)
    {
        return Ok(Some(MatchKanaKanjiResult::Yes));
    }
    // dict.lisp:1522 ((and (not stagr) (eql wtype :kanji)) nil)
    if stagr.is_empty() && wtype == WordType::Kanji {
        return Ok(None);
    }
    // dict.lisp:1523 ((and (not stagk) (eql wtype :kana)) nil)
    if stagk.is_empty() && wtype == WordType::Kana {
        return Ok(None);
    }
    // dict.lisp:1524 (restricted (query (:select 'reading 'text :from 'restricted-readings :where (:= 'seq seq))))
    let restricted: Vec<(String, String)> =
        sqlx::query_as("SELECT reading, text FROM restricted_readings WHERE seq = $1")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    // dict.lisp:1525-1532 (case wtype …)
    match wtype {
        WordType::Kanji => {
            // dict.lisp:1527 (rkana (select-dao 'kana-text (:and (:= 'seq seq) (:in 'text (:set stagr)))))
            let rkana: Vec<KanaText> =
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = ANY($2)")
                    .bind(seq)
                    .bind(stagr)
                    .fetch_all(&ctx.pool)
                    .await?;
            // dict.lisp:1528 (some (lambda (rk) (match-kana-kanji rk reading restricted)) rkana)
            Ok(rkana.into_iter().find_map(|rk| {
                match_kana_kanji(&KaniWordDispatchEnum::Kana(rk), reading, &restricted)
            }))
        }
        WordType::Kana => {
            // dict.lisp:1530 (rkanji (select-dao 'kanji-text (:and (:= 'seq seq) (:in 'text (:set stagk)))))
            let rkanji: Vec<KanjiText> =
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = ANY($2)")
                    .bind(seq)
                    .bind(stagk)
                    .fetch_all(&ctx.pool)
                    .await?;
            // dict.lisp:1531 (some (lambda (rk) (match-kana-kanji reading rk restricted)) rkanji)
            Ok(rkanji.into_iter().find_map(|rk| {
                match_kana_kanji(reading, &KaniWordDispatchEnum::Kanji(rk), &restricted)
            }))
        }
        // (case wtype …) has no :gap clause → nil
        WordType::Gap => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::get_senses_raw::get_senses_raw;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn props_of(ctx: &KaniranContext, seq: i32, ord: i32) -> Vec<(String, Vec<String>)> {
        get_senses_raw(ctx, seq)
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.ord == ord)
            .expect("sense ord present")
            .props
    }

    async fn reading_of(ctx: &KaniranContext, seq: i32, text: &str, kanji: bool) -> KaniWordDispatchEnum {
        if kanji {
            let row: KanjiText =
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
                    .bind(seq).bind(text).fetch_one(&ctx.pool).await.unwrap();
            KaniWordDispatchEnum::Kanji(row)
        } else {
            let row: KanaText =
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
                    .bind(seq).bind(text).fetch_one(&ctx.pool).await.unwrap();
            KaniWordDispatchEnum::Kana(row)
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::match-sense-restrictions`), 2026-05-24.
    /// Covers every cond clause: no-restriction (1339160 s0), direct
    /// member (出し / ダシ / 遇う / ボタボタ / 風太郎), `:kanji`-only nil
    /// (配う), `:kana`-only nil (ポタポタ), the `:kanji` select-dao branch
    /// (出端→t, 出汁→nil via the nokanji ダシ row), and the `:kana`
    /// select-dao branch returning t (だし / あしらう) and the matched
    /// kanji string (ぷうたろう / ふうたろう → "風太郎"; プーたろう → nil;
    /// プータロー nokanji → nil).
    #[tokio::test]
    async fn match_sense_restrictions_fixtures() {
        let ctx = ctx_from_env().await;

        struct Case { seq: i32, ord: i32, text: &'static str, kanji: bool, expected: Option<MatchKanaKanjiResult> }
        let yes = || Some(MatchKanaKanjiResult::Yes);
        let found = |s: &str| Some(MatchKanaKanjiResult::Found(s.to_string()));
        let cases = [
            // no stags → t (every reading)
            Case { seq: 1339160, ord: 0, text: "出し", kanji: true, expected: yes() },
            Case { seq: 1339160, ord: 0, text: "だし", kanji: false, expected: yes() },
            // both stags (stagk 出し, stagr ダシ)
            Case { seq: 1339160, ord: 1, text: "出し", kanji: true, expected: yes() },   // member stagk
            Case { seq: 1339160, ord: 1, text: "ダシ", kanji: false, expected: yes() },  // member stagr
            Case { seq: 1339160, ord: 1, text: "出汁", kanji: true, expected: None },    // :kanji branch, only ダシ(nokanji) matches → nil
            Case { seq: 1339160, ord: 1, text: "だし", kanji: false, expected: yes() },  // :kana branch → t
            // only stagk (遇う)
            Case { seq: 1000300, ord: 0, text: "遇う", kanji: true, expected: yes() },   // member stagk
            Case { seq: 1000300, ord: 0, text: "配う", kanji: true, expected: None },    // :kanji-only ∧ kanji → nil
            Case { seq: 1000300, ord: 0, text: "あしらう", kanji: false, expected: yes() }, // :kana branch → t
            // only stagr (ボタボタ / ぼたぼた)
            Case { seq: 1011960, ord: 1, text: "ボタボタ", kanji: false, expected: yes() }, // member stagr
            Case { seq: 1011960, ord: 1, text: "ポタポタ", kanji: false, expected: None },  // :kana-only ∧ kana → nil
            // both stags, :kanji branch returning t (1580140 stagr でばな non-nokanji)
            Case { seq: 1580140, ord: 0, text: "出端", kanji: true, expected: yes() },
            // only stagk (風太郎) with restricted_readings rows → Found(string)
            Case { seq: 1115120, ord: 1, text: "風太郎", kanji: true, expected: yes() },        // member stagk
            Case { seq: 1115120, ord: 1, text: "プー太郎", kanji: true, expected: None },        // :kanji-only ∧ kanji → nil
            Case { seq: 1115120, ord: 1, text: "プータロー", kanji: false, expected: None },     // nokanji kana → match-kana-kanji nil
            Case { seq: 1115120, ord: 1, text: "ぷうたろう", kanji: false, expected: found("風太郎") },
            Case { seq: 1115120, ord: 1, text: "ふうたろう", kanji: false, expected: found("風太郎") },
            Case { seq: 1115120, ord: 1, text: "プーたろう", kanji: false, expected: None },     // restr keyed プー太郎, 風太郎 absent → nil
        ];

        for case in &cases {
            let props = props_of(&ctx, case.seq, case.ord).await;
            let reading = reading_of(&ctx, case.seq, case.text, case.kanji).await;
            let actual = match_sense_restrictions(&ctx, case.seq, &props, &reading)
                .await
                .unwrap();
            assert_eq!(
                actual, case.expected,
                "seq={} ord={} text={}", case.seq, case.ord, case.text,
            );
        }
    }
}
