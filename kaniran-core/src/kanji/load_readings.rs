//! Port of `ichiran/kanji:load-readings` (`kanji.lisp:116`).
//!
//! Walks the `<reading>` element list of a kanjidic2 character,
//! filters down to the `ja_on` and `ja_kun` entries, normalises each
//! to hiragana, strips the prefix/suffix `-` markers, splits off the
//! okurigana portion at the `.` separator, deduplicates by reading
//! text (an entry seen with both `ja_on` and `ja_kun` is recorded
//! once as `ja_onkun`, with okurigana forms accumulated and the
//! suffix/prefix flags OR-ed in), and inserts one `reading` row per
//! distinct reading plus one `okurigana` row per accumulated
//! okurigana form.
//!
//! Diverges from the upstream lambda list `(nodes kanji-id)` by:
//!
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per [`crate::conn::kani_context`];
//! - taking the `<reading>` nodes as a `&[Node]` slice, mirroring how
//!   [`super::load_kanji`] passes the `dom:get-elements-by-tag-name`
//!   result into this function;
//! - returning `Result<(), sqlx::Error>` since the Lisp `do`-loop has
//!   no useful return value.
//!
//! Reading row inserts hardcode `stat_common = 0` (the
//! upstream `:initform 0` on the `stat-common` slot of `reading`, which
//! `make-dao` would otherwise apply implicitly).
//!
//! `IndexMap` keeps the deduplicated readings in kanjidic2 `<reading>`
//! order, so each row's `reading.id` is assigned in that order. The
//! ordering is load-bearing: `get_readings_cache` reads the rows back
//! `ORDER BY r.id` and `get_normal_readings` breaks ambiguous-gemination
//! ties by first occurrence, so kanjidic2 order decides which reading
//! wins. Upstream uses a `cl:hash-table` (implementation-defined order);
//! pinning the order here makes `reading.stat_common` deterministic
//! run-to-run.

use super::reading_dao::Reading;
use crate::characters::as_hiragana::as_hiragana;
use crate::conn::kani_context::KaniranContext;
use crate::dict::node_text::node_text;
use indexmap::IndexMap;
use roxmltree::Node;

struct ReadingInfo {
    okuri: Vec<String>,
    r#type: String,
    suffixp: bool,
    prefixp: bool,
}

pub async fn load_readings(
    ctx: &KaniranContext,
    nodes: &[Node<'_, '_>],
    kanji_id: i32,
) -> Result<(), sqlx::Error> {
    let mut readings: IndexMap<String, ReadingInfo> = IndexMap::new();
    // kanji.lisp:118-143 (dom:do-node-list (node nodes) …)
    for node in nodes {
        let r#type = node.attribute("r_type").unwrap_or("");
        let raw_text = node_text(*node, None);
        if !(r#type == "ja_on" || r#type == "ja_kun") {
            continue;
        }
        let mut text = as_hiragana(&raw_text);
        let mut suffixp = false;
        let mut prefixp = false;
        // kanji.lisp:124-125 (when (char= (char text 0) #\-) …)
        if text.starts_with('-') {
            suffixp = true;
            text = text[1..].to_string();
        }
        // kanji.lisp:126-127 (when (char= (char text (1- (length text))) #\-) …)
        if text.ends_with('-') {
            prefixp = true;
            let new_len = text.len() - 1;
            text.truncate(new_len);
        }
        // kanji.lisp:128-132 (let ((dot (position #\. text))) (if dot …))
        let (reading_str, okuri): (String, Option<String>) = match text.find('.') {
            Some(dot) => {
                let before = text[..dot].to_string();
                let after = text[dot + 1..].to_string();
                (before, Some(after))
            }
            None => (text.clone(), None),
        };
        // kanji.lisp:133-143 (let ((old-reading (gethash reading readings))) (cond …))
        match readings.get_mut(&reading_str) {
            Some(old) => {
                if old.r#type != r#type {
                    old.r#type = "ja_onkun".to_string();
                }
                if let Some(o) = okuri {
                    old.okuri.insert(0, o);
                }
                if suffixp {
                    old.suffixp = true;
                }
                if prefixp {
                    old.prefixp = true;
                }
            }
            None => {
                let okuri_vec = match okuri {
                    Some(o) => vec![o],
                    None => Vec::new(),
                };
                readings.insert(
                    reading_str,
                    ReadingInfo {
                        okuri: okuri_vec,
                        r#type: r#type.to_string(),
                        suffixp,
                        prefixp,
                    },
                );
            }
        }
    }
    // kanji.lisp:144-152 (maphash (lambda (text rinfo) …) readings)
    for (text, info) in readings {
        // kanji.lisp:148 (apply #'make-dao 'reading :text text :kanji-id kanji-id rinfo)
        // kanji.lisp:50-51 — reading.stat-common :initform 0
        let robj: Reading = sqlx::query_as(
            "INSERT INTO reading (kanji_id, type, text, suffixp, prefixp, stat_common) \
             VALUES ($1, $2, $3, $4, $5, 0) \
             RETURNING id, kanji_id, type, text, suffixp, prefixp, stat_common",
        )
        .bind(kanji_id)
        .bind(&info.r#type)
        .bind(&text)
        .bind(info.suffixp)
        .bind(info.prefixp)
        .fetch_one(&ctx.pool)
        .await?;
        // kanji.lisp:149-151 (loop with rid = (id robj) for of in okuri do …)
        for of in &info.okuri {
            sqlx::query("INSERT INTO okurigana (reading_id, text) VALUES ($1, $2)")
                .bind(robj.id)
                .bind(of)
                .execute(&ctx.pool)
                .await?;
        }
    }
    Ok(())
}
