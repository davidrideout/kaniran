//! Port of `ichiran/dict:add-deha-ja-readings` (`dict-errata.lisp:173`).
//!
//! For every conjugation derived from seq 2089020 (the copula `で[は]`)
//! whose kana reading starts with `では`, mints a sibling reading
//! where `では` is rewritten to `じゃ`. Same rewrite is applied to the
//! matching `conj_source_reading` rows; `source_text` itself is
//! rewritten only when it starts with `では`.

use super::add_reading::add_reading;
use super::conj_source_reading_dao::ConjSourceReading;
use crate::conn::kani_context::KaniranContext;

/// `(concatenate 'string "じゃ" (subseq deha 2))`.
fn rewrite_deha_to_ja(s: &str) -> String {
    let split = s
        .char_indices()
        .nth(2)
        .map(|(b, _)| b)
        .unwrap_or(s.len());
    format!("じゃ{}", &s[split..])
}

pub async fn add_deha_ja_readings(
    ctx: &KaniranContext,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:174-178 (query (:select 'conj.seq 'kt.text :distinct …))
    let deha_list: Vec<(i32, String)> = sqlx::query_as(
        r#"SELECT DISTINCT conj.seq, kt.text
           FROM conjugation AS conj, kana_text AS kt
           WHERE conj."from" = 2089020
             AND kt.seq = conj.seq
             AND kt.text LIKE 'では%'"#,
    )
    .fetch_all(&ctx.pool)
    .await?;
    // dict-errata.lisp:179-181 (loop for (seq deha) … do (add-reading seq ja))
    for (seq, deha) in &deha_list {
        let ja = rewrite_deha_to_ja(deha);
        add_reading(ctx, *seq, &ja, None, true, None).await?;
    }

    // dict-errata.lisp:183-187 (query (:select 'csr.conj-id 'csr.text 'csr.source-text :distinct? — NO …))
    let deha_src_reading: Vec<(i32, String, String)> = sqlx::query_as(
        r#"SELECT csr.conj_id, csr.text, csr.source_text
           FROM conjugation AS conj, conj_source_reading AS csr
           WHERE conj."from" = 2089020
             AND csr.conj_id = conj.id
             AND csr.text LIKE 'では%'"#,
    )
    .fetch_all(&ctx.pool)
    .await?;
    // dict-errata.lisp:188-196 (loop for (conj-id text source-text) … unless jsr do (make-dao …))
    for (conj_id, text, source_text) in &deha_src_reading {
        let ja = rewrite_deha_to_ja(text);
        // dict-errata.lisp:190 (select-dao 'conj-source-reading (:and (:= 'conj-id conj-id) (:= 'text ja) (:= 'source-text source-text)))
        let jsr: Vec<ConjSourceReading> = sqlx::query_as(
            "SELECT * FROM conj_source_reading \
             WHERE conj_id = $1 AND text = $2 AND source_text = $3",
        )
        .bind(conj_id)
        .bind(&ja)
        .bind(source_text)
        .fetch_all(&ctx.pool)
        .await?;
        if !jsr.is_empty() {
            continue;
        }
        // dict-errata.lisp:194-196 (:source-text (if (alexandria:starts-with-subseq "では" source-text) (concatenate … "じゃ" (subseq … 2)) source-text))
        let new_source_text = if source_text.starts_with("では") {
            rewrite_deha_to_ja(source_text)
        } else {
            source_text.clone()
        };
        // dict-errata.lisp:192-196 (make-dao 'conj-source-reading :conj-id conj-id :text ja :source-text …)
        sqlx::query(
            "INSERT INTO conj_source_reading (conj_id, text, source_text) \
             VALUES ($1, $2, $3)",
        )
        .bind(conj_id)
        .bind(&ja)
        .bind(&new_source_text)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL pin (`(concatenate 'string "じゃ" (subseq input 2))`),
    /// 2026-05-31. Cases drawn from the live `では`-prefixed kana_text
    /// join inside `add_deha_ja_readings`, plus the exactly-2-char
    /// boundary `では`.
    #[test]
    fn rewrite_deha_to_ja_cases() {
        let cases: &[(&str, &str)] = &[
            ("ではない", "じゃない"),
            ("ではなかった", "じゃなかった"),
            ("ではありませんでした", "じゃありませんでした"),
            ("ではないで", "じゃないで"),
            ("ではなくて", "じゃなくて"),
            ("ではなかったら", "じゃなかったら"),
            ("ではありませんでしたら", "じゃありませんでしたら"),
            ("ではありません", "じゃありません"),
            ("ではないです", "じゃないです"),
            ("では", "じゃ"),
        ];
        for (input, expected) in cases {
            assert_eq!(&rewrite_deha_to_ja(input), expected, "input={input}");
        }
    }
}
