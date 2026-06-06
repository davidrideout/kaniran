//! Port of `ichiran/dict:insert-readings` (`dict-load.lisp:32`).
//!
//! Inserts the kanji or kana readings for one entry. Skips readings
//! tagged `re_inf=ok`, records `re_restr` restrictions, then updates
//! the parent entry's reading count and `primary_nokanji` flag.

use crate::conn::kani_context::KaniranContext;
use crate::dict::node_text::node_text;
use roxmltree::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingTable {
    KanaText,
    KanjiText,
}

impl ReadingTable {
    fn insert_sql(self) -> &'static str {
        // Hardcoded defaults match the dict.lisp:86/128 initforms:
        // conjugate-p = TRUE, best-kana/best-kanji = NULL.
        match self {
            ReadingTable::KanaText => {
                "INSERT INTO kana_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kanji) \
                 VALUES ($1, $2, $3, $4, $5, TRUE, $6, NULL)"
            }
            ReadingTable::KanjiText => {
                "INSERT INTO kanji_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kana) \
                 VALUES ($1, $2, $3, $4, $5, TRUE, $6, NULL)"
            }
        }
    }

    fn update_sql(self) -> &'static str {
        match self {
            ReadingTable::KanaText => {
                "UPDATE entry SET primary_nokanji = $1, n_kana = $2 WHERE seq = $3"
            }
            ReadingTable::KanjiText => {
                "UPDATE entry SET primary_nokanji = $1, n_kanji = $2 WHERE seq = $3"
            }
        }
    }
}

pub async fn insert_readings(
    ctx: &KaniranContext,
    node_list: &[Node<'_, '_>],
    tag: &str,
    table: ReadingTable,
    seq: i32,
    pri: &str,
) -> Result<(), sqlx::Error> {
    let mut to_add: Vec<(String, Option<i32>, bool, String)> = Vec::new();
    let mut primary_nokanji = false;

    for node in node_list.iter() {
        let reading_node = node
            .descendants()
            .find(|n| *n != *node && n.is_element() && n.has_tag_name(tag))
            .expect("insert_readings: missing reading element");
        let reading_text = node_text(reading_node, None);
        let mut common: Option<i32> = None;
        let mut skip = false;
        let mut nokanji = false;
        let mut pri_tags: Vec<String> = Vec::new();

        for re_inf in node
            .descendants()
            .filter(|n| *n != *node && n.is_element() && n.has_tag_name("re_inf"))
        {
            if node_text(re_inf, None) == "ok" {
                skip = true;
            }
        }
        if !skip {
            if node
                .descendants()
                .any(|n| n != *node && n.is_element() && n.has_tag_name("re_nokanji"))
            {
                nokanji = true;
            }
            for re_restr in node
                .descendants()
                .filter(|n| *n != *node && n.is_element() && n.has_tag_name("re_restr"))
            {
                let restr = node_text(re_restr, None);
                sqlx::query(
                    "INSERT INTO restricted_readings (seq, reading, text) \
                     VALUES ($1, $2, $3)",
                )
                .bind(seq)
                .bind(&reading_text)
                .bind(&restr)
                .execute(&ctx.pool)
                .await?;
            }
            for pri_node in node
                .descendants()
                .filter(|n| *n != *node && n.is_element() && n.has_tag_name(pri))
            {
                let pri_tag = node_text(pri_node, None);
                if common.is_none() {
                    common = Some(0);
                }
                if let Some(rest) = pri_tag.strip_prefix("nf") {
                    common = Some(
                        rest.parse::<i32>()
                            .expect("insert_readings: malformed nf pri tag"),
                    );
                }
                pri_tags.push(pri_tag);
            }
            let mut common_tags = String::new();
            for pri_tag in &pri_tags {
                common_tags.push('[');
                common_tags.push_str(pri_tag);
                common_tags.push(']');
            }
            to_add.push((reading_text, common, nokanji, common_tags));
        }
    }

    let n_added = to_add.len() as i32;
    for (ord, (reading_text, common, nokanji, common_tags)) in to_add.iter().enumerate() {
        if *nokanji {
            primary_nokanji = true;
        }
        sqlx::query(table.insert_sql())
            .bind(seq)
            .bind(reading_text)
            .bind(ord as i32)
            .bind(*common)
            .bind(common_tags)
            .bind(*nokanji)
            .execute(&ctx.pool)
            .await?;
    }

    sqlx::query(table.update_sql())
        .bind(primary_nokanji)
        .bind(n_added)
        .bind(seq)
        .execute(&ctx.pool)
        .await?;

    Ok(())
}
