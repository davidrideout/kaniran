//! Port of `ichiran/kanji:load-kanji` (`kanji.lisp:155`).
//!
//! Parses one kanjidic2 `<character>` XML payload, inserts the `kanji`
//! row and its associated `reading`, `nanori`-style reading
//! (`type = "ja_na"`), and English `meaning` rows. The `<reading>`
//! list is handed off to [`super::load_readings`] for dedup +
//! okurigana extraction; `<nanori>` and `<meaning>` are inserted in
//! the order they appear in the source.
//!
//! `content` is the serialized XML for a single `<character>` element
//! (the form produced by [`super::load_kanjidic::serialize_character`]
//! when streaming the kanjidic2 dump).
//!
//! Diverges from the upstream lambda list `(content)` by:
//!
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per [`crate::conn::kani_context`];
//! - returning `Result<(), sqlx::Error>` since the Lisp form's
//!   trailing `do-node-list` has no useful return value.
//!
//! Kanji row inserts hardcode `stat_common = 0` and
//! `stat_irregular = 0` (the upstream `:initform 0` on those slots,
//! which `make-dao` would otherwise apply implicitly). Nanori rows go
//! through the `reading` table with `suffixp = false`, `prefixp =
//! false`, `stat_common = 0` (the `reading` slot initforms).

use super::load_readings::load_readings;
use crate::characters::as_hiragana::as_hiragana;
use crate::conn::kani_context::KaniranContext;
use crate::dict::node_text::node_text;
use roxmltree::{Document, Node};

pub async fn load_kanji(
    ctx: &KaniranContext,
    content: &str,
) -> Result<(), sqlx::Error> {
    // kanji.lisp:156 (parsed (cxml:parse content (cxml-dom:make-dom-builder)))
    let parsed = Document::parse(content).expect("load_kanji: malformed character XML");
    let root = parsed.root_element();
    let descendants: Vec<Node<'_, '_>> =
        root.descendants().filter(|n| n.is_element()).collect();
    let by_tag = |tag: &str| -> Vec<Node<'_, '_>> {
        descendants
            .iter()
            .copied()
            .filter(|n| n.has_tag_name(tag))
            .collect()
    };
    // kanji.lisp:157 (literal (first-node-text (dom:get-elements-by-tag-name parsed "literal")))
    let literal_nodes = by_tag("literal");
    let literal: String = if literal_nodes.is_empty() {
        panic!("load_kanji: missing <literal>");
    } else {
        node_text(literal_nodes[0], None)
    };
    // kanji.lisp:158 (node-radical (dom:get-elements-by-tag-name parsed "rad_value"))
    let node_radical = by_tag("rad_value");
    // kanji.lisp:160-162 (grade …)
    let grade_nodes = by_tag("grade");
    let grade: Option<i32> = if grade_nodes.is_empty() {
        None
    } else {
        Some(
            node_text(grade_nodes[0], None)
                .parse::<i32>()
                .expect("load_kanji: malformed <grade>"),
        )
    };
    // kanji.lisp:162-163 (strokes …)
    let stroke_nodes = by_tag("stroke_count");
    let strokes: Option<i32> = if stroke_nodes.is_empty() {
        None
    } else {
        Some(
            node_text(stroke_nodes[0], None)
                .parse::<i32>()
                .expect("load_kanji: malformed <stroke_count>"),
        )
    };
    let strokes: i32 = strokes.expect("load_kanji: missing <stroke_count>");
    // kanji.lisp:164-165 (freq …)
    let freq_nodes = by_tag("freq");
    let freq: Option<i32> = if freq_nodes.is_empty() {
        None
    } else {
        Some(
            node_text(freq_nodes[0], None)
                .parse::<i32>()
                .expect("load_kanji: malformed <freq>"),
        )
    };
    // kanji.lisp:166-168 (node-reading / node-nanori / node-meaning)
    let node_reading = by_tag("reading");
    let node_nanori = by_tag("nanori");
    let node_meaning = by_tag("meaning");
    // kanji.lisp:169-174 (dom:do-node-list (node node-radical) …)
    let mut radical_c: Option<i32> = None;
    let mut radical_n: Option<i32> = None;
    for node in &node_radical {
        let r#type = node.attribute("rad_type").unwrap_or("");
        let radical: i32 = node_text(*node, None)
            .parse::<i32>()
            .expect("load_kanji: malformed <rad_value>");
        // kanji.lisp:172-174 ((equal type "classical") … ((equal type "nelson_c") …))
        if r#type == "classical" {
            if radical_c.is_none() {
                radical_c = Some(radical);
            }
        } else if r#type == "nelson_c" {
            if radical_n.is_none() {
                radical_n = Some(radical);
            }
        }
    }
    // kanji.lisp:175 (unless radical-n (setf radical-n radical-c))
    if radical_n.is_none() {
        radical_n = radical_c;
    }
    let radical_c: i32 = radical_c.expect("load_kanji: missing classical rad_value");
    let radical_n: i32 = radical_n.expect("load_kanji: missing nelson_c/classical rad_value");
    // kanji.lisp:176-177 (let ((kanji-id (id (make-dao 'kanji …))))
    // kanji.lisp:19-22 — kanji.stat-common / stat-irregular :initform 0
    let kanji_id: i32 = sqlx::query_scalar(
        "INSERT INTO kanji (text, radical_c, radical_n, grade, strokes, freq, stat_common, stat_irregular) \
         VALUES ($1, $2, $3, $4, $5, $6, 0, 0) \
         RETURNING id",
    )
    .bind(&literal)
    .bind(radical_c)
    .bind(radical_n)
    .bind(grade)
    .bind(strokes)
    .bind(freq)
    .fetch_one(&ctx.pool)
    .await?;
    // kanji.lisp:178 (load-readings node-reading kanji-id)
    load_readings(ctx, &node_reading, kanji_id).await?;
    // kanji.lisp:179-180 (dom:do-node-list (node node-nanori) …)
    // kanji.lisp:47-51 — reading.suffixp/prefixp :initform nil, stat-common :initform 0
    for node in &node_nanori {
        let nanori_text = as_hiragana(&node_text(*node, None));
        sqlx::query(
            "INSERT INTO reading (kanji_id, type, text, suffixp, prefixp, stat_common) \
             VALUES ($1, 'ja_na', $2, FALSE, FALSE, 0)",
        )
        .bind(kanji_id)
        .bind(&nanori_text)
        .execute(&ctx.pool)
        .await?;
    }
    // kanji.lisp:181-185 (dom:do-node-list (node node-meaning) …)
    for node in &node_meaning {
        let lang = node.attribute("m_lang").unwrap_or("");
        let text = node_text(*node, None);
        // kanji.lisp:184 (when (member lang '("" "en") :test 'equal) …)
        if lang.is_empty() || lang == "en" {
            sqlx::query("INSERT INTO meaning (kanji_id, text) VALUES ($1, $2)")
                .bind(kanji_id)
                .bind(&text)
                .execute(&ctx.pool)
                .await?;
        }
    }
    Ok(())
}
