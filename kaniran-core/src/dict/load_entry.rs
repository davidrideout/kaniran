//! Port of `ichiran/dict:load-entry` (`dict-load.lisp:115`).
//!
//! Parses one JMdict `<entry>` XML element, inserts the `entry` row
//! plus its `kanji_text`, `kana_text`, and `sense` children, and
//! optionally conjugates verbs/adjectives. Returns `Some(seq)` on
//! success, `None` on `:skip` / upstream-text-match early-return.
//!
//! `content` is the serialized XML for a single entry (one `<entry>`
//! element, entities already expanded by [`fix_entities`]). The
//! upstream `content` parameter also accepted a `dom:node`; the Rust
//! port keeps the [`&str`] case only — every reachable callsite in
//! `load-jmdict` passes a serialized string.
//!
//! [`fix_entities`]: super::fix_entities

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_pos_with_conj_rules_star_::POS_WITH_CONJ_RULES;
use crate::dict::conjugate_entry_outer::conjugate_entry_outer;
use crate::dict::entry_dao::Entry;
use crate::dict::find_word::{find_word, FindWordRows};
use crate::dict::insert_readings::{insert_readings, ReadingTable};
use crate::dict::insert_senses::insert_senses;
use crate::dict::load_secondary_conjugations::load_secondary_conjugations;
use crate::dict::next_seq::next_seq;
use crate::dict::node_text::node_text;
use roxmltree::Document;

pub enum LoadEntryIfExists {
    None,
    Skip,
    Overwrite,
}

pub enum LoadEntrySeq<'a> {
    None,
    Str(&'a str),
    Int(i32),
}

pub async fn load_entry(
    ctx: &KaniranContext,
    content: &str,
    if_exists: LoadEntryIfExists,
    upstream: Option<(i32, &str)>,
    seq: LoadEntrySeq<'_>,
    conjugate_p: bool,
) -> Result<Option<i32>, sqlx::Error> {
    // dict-load.lisp:116-122 (parsed (typecase content ...))
    let parsed = Document::parse(content)
        .expect("load_entry: malformed entry XML");
    // dict-load.lisp:123-132 (seq (cond ...))
    let seq: i32 = match seq {
        LoadEntrySeq::Str(s) => {
            match find_word(ctx, s, false).await? {
                FindWordRows::Kana(rows) => match rows.first() {
                    Some(row) => row.seq,
                    None => next_seq(ctx).await?,
                },
                FindWordRows::Kanji(rows) => match rows.first() {
                    Some(row) => row.seq,
                    None => next_seq(ctx).await?,
                },
            }
        }
        LoadEntrySeq::Int(n) => n,
        LoadEntrySeq::None => {
            // dict-load.lisp:131-132 (let ((entseq-node (dom:item (dom:get-elements-by-tag-name parsed "ent_seq") 0)))
            //                         (parse-integer (node-text entseq-node)))
            let entseq_node = parsed
                .descendants()
                .find(|n| n.is_element() && n.has_tag_name("ent_seq"))
                .expect("load_entry: missing ent_seq element");
            node_text(entseq_node, None)
                .parse::<i32>()
                .expect("load_entry: malformed ent_seq")
        }
    };
    // dict-load.lisp:133-136 (when upstream (let ((entry (get-dao 'entry (car upstream))))
    //                          (when (and entry (equal (get-text entry) (cadr upstream)))
    //                            (return-from load-entry))))
    if let Some((up_seq, up_text)) = upstream {
        let entry: Option<Entry> =
            sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
                .bind(up_seq)
                .fetch_optional(&ctx.pool)
                .await?;
        if let Some(entry) = entry {
            if let Some(text) = entry.get_text(ctx).await? {
                if text == up_text {
                    return Ok(None);
                }
            }
        }
    }
    // dict-load.lisp:137-140 (case if-exists (:skip ...) (:overwrite ...))
    match if_exists {
        LoadEntryIfExists::Skip => {
            let exists: Option<i32> =
                sqlx::query_scalar("SELECT seq FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_optional(&ctx.pool)
                    .await?;
            if exists.is_some() {
                return Ok(None);
            }
        }
        LoadEntryIfExists::Overwrite => {
            sqlx::query("DELETE FROM entry WHERE seq = $1")
                .bind(seq)
                .execute(&ctx.pool)
                .await?;
        }
        LoadEntryIfExists::None => {}
    }
    // dict-load.lisp:142 (make-dao 'entry :seq seq :content content :root-p t)
    // dict.lisp:26-35 entry initforms: root-p nil (overridden true), n-kanji 0,
    // n-kana 0, primary-nokanji nil.
    sqlx::query(
        "INSERT INTO entry (seq, content, root_p, n_kanji, n_kana, primary_nokanji) \
         VALUES ($1, $2, TRUE, 0, 0, FALSE)",
    )
    .bind(seq)
    .bind(content)
    .execute(&ctx.pool)
    .await?;

    // dict-load.lisp:143-148 (let* ((kanji-nodes ...) (kana-nodes ...) (sense-nodes ...)) ...)
    let kanji_nodes: Vec<_> = parsed
        .descendants()
        .filter(|n| n.is_element() && n.has_tag_name("k_ele"))
        .collect();
    let kana_nodes: Vec<_> = parsed
        .descendants()
        .filter(|n| n.is_element() && n.has_tag_name("r_ele"))
        .collect();
    let sense_nodes: Vec<_> = parsed
        .descendants()
        .filter(|n| n.is_element() && n.has_tag_name("sense"))
        .collect();
    insert_readings(ctx, &kanji_nodes, "keb", ReadingTable::KanjiText, seq, "ke_pri").await?;
    insert_readings(ctx, &kana_nodes, "reb", ReadingTable::KanaText, seq, "re_pri").await?;
    insert_senses(ctx, &sense_nodes, seq).await?;

    // dict-load.lisp:149-158 (when conjugate-p ...)
    if conjugate_p {
        let posi: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT text FROM sense_prop \
             WHERE seq = $1 AND tag = 'pos' AND text = ANY($2)",
        )
        .bind(seq)
        .bind(POS_WITH_CONJ_RULES)
        .fetch_all(&ctx.pool)
        .await?;
        if !posi.is_empty() {
            conjugate_entry_outer(ctx, seq, None, None, Some(&posi)).await?;
            load_secondary_conjugations(ctx, Some(&[seq])).await?;
        }
    }
    Ok(Some(seq))
}
