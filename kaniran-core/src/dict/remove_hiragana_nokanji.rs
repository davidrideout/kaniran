//! Port of `ichiran/dict:remove-hiragana-nokanji` (`dict-errata.lisp:217`).
//!
//! Finds every `kana_text` row carrying `nokanji = TRUE` whose `text`
//! is pure hiragana, then clears the `primary_nokanji` flag on every
//! entry that owns one of those rows and still has the flag set.

use super::entry_dao::Entry;
use super::kana_text_dao::KanaText;
use crate::characters::char_class::CharClass;
use crate::characters::char_class::test_word;
use crate::conn::kani_context::KaniranContext;

pub async fn remove_hiragana_nokanji(
    ctx: &KaniranContext,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:218-219 (remove-if-not … (select-dao 'kana-text 'nokanji))
    let all_nokanji_kts: Vec<KanaText> =
        sqlx::query_as("SELECT * FROM kana_text WHERE nokanji")
            .fetch_all(&ctx.pool)
            .await?;
    let kts: Vec<KanaText> = all_nokanji_kts
        .into_iter()
        .filter(|kt| test_word(&kt.text, CharClass::Hiragana))
        .collect();
    // dict-errata.lisp:220 (select-dao 'entry (:and (:in 'seq (:set (mapcar #'seq kts))) 'primary-nokanji))
    if kts.is_empty() {
        return Ok(());
    }
    let seqs: Vec<i32> = kts.iter().map(|kt| kt.seq).collect();
    let entries: Vec<Entry> = sqlx::query_as(
        "SELECT * FROM entry WHERE seq = ANY($1) AND primary_nokanji",
    )
    .bind(&seqs)
    .fetch_all(&ctx.pool)
    .await?;
    // dict-errata.lisp:221-222 (setf (slot-value entry 'primary-nokanji) nil) (update-dao entry)
    for mut entry in entries {
        entry.primary_nokanji = false;
        sqlx::query(
            "UPDATE entry SET content = $2, root_p = $3, n_kanji = $4, \
             n_kana = $5, primary_nokanji = $6 WHERE seq = $1",
        )
        .bind(entry.seq)
        .bind(&entry.content)
        .bind(entry.root_p)
        .bind(entry.n_kanji)
        .bind(entry.n_kana)
        .bind(entry.primary_nokanji)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(())
}
