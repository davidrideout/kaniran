//! Port of `ichiran/dict:get-senses-json` (`dict.lisp:1537`).
//!
//! Builds the per-sense JSON objects (`pos` / `gloss` plus optional
//! `field` and `info`) for an entry, filtering by `pos_list` and, when
//! a reading is supplied, by sense restrictions. The `reading_getter`
//! thunk is awaited at most once across the loop.

use std::future::Future;

use serde_json::{Map, Value};

use crate::characters::join::join;
use crate::conn::kani_context::KaniranContext;

use super::get_senses::get_senses;
use super::kani_word::KaniWordDispatchEnum;
use super::match_sense_restrictions::match_sense_restrictions;
use super::split_pos::split_pos;

pub async fn get_senses_json<Fut>(
    ctx: &KaniranContext,
    seq: i32,
    pos_list: &[String],
    reading: Option<KaniWordDispatchEnum>,
    reading_getter: Option<Fut>,
) -> Result<Vec<Value>, sqlx::Error>
where
    Fut: Future<Output = Result<Option<KaniWordDispatchEnum>, sqlx::Error>>,
{
    let has_reading_getter = reading_getter.is_some();
    let mut reading_getter = reading_getter;
    let mut reading = reading;
    let mut readp = false;
    let mut rpos = String::new();
    let mut lpos: Vec<String> = Vec::new();
    let mut first = true;
    let mut out: Vec<Value> = Vec::new();

    for (pos, gloss, props) in get_senses(ctx, seq).await? {
        let emptypos = pos == "[]";
        // for rpos / lpos = … then (if emptypos … …): first iteration uses
        // the raw value, later iterations keep the prior on an empty pos.
        if first || !emptypos {
            rpos = pos.clone();
            lpos = split_pos(&pos).into_iter().map(str::to_owned).collect();
            first = false;
        }
        let rinf = props
            .iter()
            .find(|(tag, _)| tag == "s_inf")
            .map(|(_, inf)| join("; ", inf));
        let rfield = props
            .iter()
            .find(|(tag, _)| tag == "field")
            .map(|(_, field)| format!("{{{}}}", field.join(",")));

        // (or (not pos-list) (intersection lpos pos-list :test 'equal))
        let cond1 =
            pos_list.is_empty() || lpos.iter().any(|lp| pos_list.iter().any(|q| q == lp));
        let collect_this = if !cond1 {
            false
        } else if !(has_reading_getter || reading.is_some()) {
            // (not (or reading-getter reading))
            true
        } else if !props.iter().any(|(tag, _)| tag == "stagk" || tag == "stagr") {
            // (not (or (assoc "stagk" props) (assoc "stagr" props)))
            true
        } else {
            // (let ((rr (or reading (and (not readp) (setf readp t reading (funcall reading-getter)))))) …)
            if reading.is_none() && !readp {
                readp = true;
                reading = match reading_getter.take() {
                    Some(fut) => fut.await?,
                    None => None,
                };
            }
            // (if rr (match-sense-restrictions seq props rr) t)
            match &reading {
                Some(rr) => match_sense_restrictions(ctx, seq, &props, rr)
                    .await?
                    .is_some(),
                None => true,
            }
        };

        if collect_this {
            let mut js = Map::new();
            js.insert("pos".to_owned(), Value::String(rpos.clone()));
            js.insert("gloss".to_owned(), Value::String(gloss));
            if let Some(rfield) = rfield {
                js.insert("field".to_owned(), Value::String(rfield));
            }
            if let Some(rinf) = rinf {
                js.insert("info".to_owned(), Value::String(rinf));
            }
            out.push(Value::Object(js));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use std::future::Ready;
    use std::sync::Arc;

    type GetterFut = Ready<Result<Option<KaniWordDispatchEnum>, sqlx::Error>>;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    async fn kanji_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanjiText = sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        KaniWordDispatchEnum::Kanji(row)
    }

    async fn kana_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanaText = sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        KaniWordDispatchEnum::Kana(row)
    }

    /// REPL fixtures (.103, `(jsown:to-json (get-senses-json …))`),
    /// 2026-05-24. No reading/getter, no pos-list: every sense is
    /// collected. Covers `field` ({food}), multi-pos `[adj-no,n]`, the
    /// `[]`-second-sense `rpos` carry-forward (1447690 → both `[n]`), and
    /// the `s_inf` → `info` path with non-ASCII text (serde emits raw
    /// UTF-8, not jsown's `\u` escapes).
    #[tokio::test]
    async fn plain_collect_all() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (1001390, r#"[{"pos":"[n]","gloss":"oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi","field":"{food}"}]"#),
            (1577900, r#"[{"pos":"[adj-no,n]","gloss":"eternity"}]"#),
            (1447690, r#"[{"pos":"[n]","gloss":"Tokyo"},{"pos":"[n]","gloss":"Tokyo Metropolis"}]"#),
            (1000230, r#"[{"pos":"[exp]","gloss":"useless; no good; hopeless","info":"commonly used with i-adjective inflections, e.g. あかんかった, あかんくない"},{"pos":"[exp]","gloss":"cannot; must not; not allowed"}]"#),
            (1000320, r#"[{"pos":"[pn]","gloss":"there; over there; that place; yonder; you-know-where","info":"place physically distant from both speaker and listener"},{"pos":"[n]","gloss":"genitals; private parts; nether regions"},{"pos":"[n]","gloss":"that far; that much; that point","info":"something psychologically distant from both speaker and listener"}]"#),
        ];
        for (seq, expected) in cases {
            let result = get_senses_json(&ctx, *seq, &[], None, None::<GetterFut>)
                .await
                .unwrap();
            assert_eq!(json(&result), *expected, "seq={seq}");
        }
    }

    /// REPL fixtures (.103), 2026-05-24. `pos-list` filter against the
    /// carried-forward `lpos`. 1577900 keeps/drops on `n`/`xxx`; 1447690
    /// `n` keeps both senses (the `[]` sense inherits `lpos=["n"]`);
    /// 1000320 `n` drops the leading `pn` sense; 1199330 `ctr` keeps only
    /// the counter sense (mirrors the `:pos-list '("ctr")` call site).
    #[tokio::test]
    async fn pos_list_filter() {
        let ctx = ctx_from_env().await;
        struct Case { seq: i32, pos: Vec<String>, expected: &'static str }
        let cases = [
            Case { seq: 1577900, pos: vec!["n".to_owned()], expected: r#"[{"pos":"[adj-no,n]","gloss":"eternity"}]"# },
            Case { seq: 1577900, pos: vec!["xxx".to_owned()], expected: "[]" },
            Case { seq: 1447690, pos: vec!["n".to_owned()], expected: r#"[{"pos":"[n]","gloss":"Tokyo"},{"pos":"[n]","gloss":"Tokyo Metropolis"}]"# },
            Case { seq: 1000320, pos: vec!["n".to_owned()], expected: r#"[{"pos":"[n]","gloss":"genitals; private parts; nether regions"},{"pos":"[n]","gloss":"that far; that much; that point","info":"something psychologically distant from both speaker and listener"}]"# },
            Case { seq: 1199330, pos: vec!["ctr".to_owned()], expected: r#"[{"pos":"[ctr]","gloss":"counter for occurrences"}]"# },
        ];
        for case in &cases {
            let result = get_senses_json(&ctx, case.seq, &case.pos, None, None::<GetterFut>)
                .await
                .unwrap();
            assert_eq!(json(&result), case.expected, "seq={} pos={:?}", case.seq, case.pos);
        }
    }

    /// REPL fixtures (.103), 2026-05-24. `:reading` restriction path
    /// (1339160 sense 1 has stagk 出し / stagr ダシ; 1115120 sense 1 has
    /// stagk 風太郎). A `stagk`/`stagr` member passes (出し / ダシ); a
    /// non-matching kanji is filtered (出汁 → nil; プー太郎 → nil); the
    /// restricted-reading `Found` path passes (ぷうたろう → 風太郎).
    #[tokio::test]
    async fn reading_restriction() {
        let ctx = ctx_from_env().await;
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let one_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"}]"#;
        let both_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"},{"pos":"[n]","gloss":"day labourer (esp. on the docks)"}]"#;

        // 出汁 (kanji): sense 1 filtered (only ダシ nokanji matches → nil)
        let reading = kanji_reading(&ctx, 1339160, "出汁").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>).await.unwrap();
        assert_eq!(json(&result), one_dashi, "1339160 出汁");
        // 出し (kanji): member of stagk → both pass
        let reading = kanji_reading(&ctx, 1339160, "出し").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>).await.unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 出し");
        // ダシ (kana): member of stagr → both pass
        let reading = kana_reading(&ctx, 1339160, "ダシ").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>).await.unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 ダシ");
        // プー太郎 (kanji): sense 1 filtered
        let reading = kanji_reading(&ctx, 1115120, "プー太郎").await;
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>).await.unwrap();
        assert_eq!(json(&result), one_taro, "1115120 プー太郎");
        // ぷうたろう (kana): match-kana-kanji Found(風太郎) → sense 1 passes
        let reading = kana_reading(&ctx, 1115120, "ぷうたろう").await;
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>).await.unwrap();
        assert_eq!(json(&result), both_taro, "1115120 ぷうたろう");
    }

    /// REPL fixtures (.103), 2026-05-24. `:reading-getter` lazy thunk.
    /// A getter yielding 出汁 filters sense 1 exactly like the eager
    /// `:reading` form; a getter yielding `nil` leaves the restricted
    /// sense in (the `(if rr … t)` fallthrough). 1011960 carries two
    /// stag-restricted senses (1 and 2): the nil getter fires once at
    /// sense 1 then sense 2 takes the `readp`-already-true / `reading`-nil
    /// path; the ぼたぼた getter fires once then sense 2 reuses the memoized
    /// `reading` (the `(or reading …)` short-circuit) — both keep all three.
    #[tokio::test]
    async fn reading_getter_path() {
        let ctx = ctx_from_env().await;
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let all_bota = r#"[{"pos":"[adv,adv-to,vs]","gloss":"dripping; trickling; drop by drop; in drops"},{"pos":"[adv,adv-to,vs]","gloss":"wet and heavy (snow, clay, etc.)"},{"pos":"[adv,adv-to]","gloss":"(moving) slowly"}]"#;

        // getter → 出汁: sense 1 filtered
        let reading = kanji_reading(&ctx, 1339160, "出汁").await;
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter)).await.unwrap();
        assert_eq!(json(&result), one_dashi, "getter 出汁");

        // getter → nil: restricted sense passes
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter)).await.unwrap();
        assert_eq!(json(&result), both_dashi, "getter nil");

        // two stag senses, nil getter: readp-true path on sense 2
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter)).await.unwrap();
        assert_eq!(json(&result), all_bota, "getter nil, two stag senses");

        // two stag senses, ぼたぼた getter (member of both): memoized reading reused
        let reading = kana_reading(&ctx, 1011960, "ぼたぼた").await;
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter)).await.unwrap();
        assert_eq!(json(&result), all_bota, "getter ぼたぼた, two stag senses");
    }
}
