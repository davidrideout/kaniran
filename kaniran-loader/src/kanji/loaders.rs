use kaniran_core::kanji::dao::{Kanji, Reading};
use kaniran_core::characters::kana::as_hiragana;
use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::kanji::stats::kanji_word_stats;
use crate::dict::load::jmdict::node_text;
use indexmap::IndexMap;
use roxmltree::{Document, Node, NodeType, ParsingOptions};
use std::path::Path;

/// Port of `ichiran/kanji:*kanjidic-path*` (`kanji.lisp:3`).
///
/// Filesystem path to the kanjidic2.xml dump consumed by
/// `load-kanjidic` (the upstream per-deployment placeholder).
pub static KANJIDIC_PATH: &str = "e:/dump/kanjidic2.xml";

/// Port of `ichiran/kanji:init-tables` (`kanji.lisp:100`).
///
/// Empties the four kanji tables (kanji, reading, okurigana, meaning)
/// and resets their identity sequences before a corpus load.
pub const TABLE_NAMES: &[&str] = &["kanji", "reading", "okurigana", "meaning"];

pub async fn init_tables(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    sqlx::query("TRUNCATE kanji, reading, okurigana, meaning RESTART IDENTITY CASCADE")
        .execute(ctx.pool.as_ref().expect("postgres pool"))
        .await?;
    Ok(())
}

/// Port of `ichiran/kanji:first-node-text` (`kanji.lisp:108`).
///
/// Returns the text of the first XML node in `nodes` (run through
/// `wrapper`), or the default when the list is empty.
pub fn first_node_text<'a, 'input, T>(
    nodes: &[Node<'a, 'input>],
    wrapper: impl FnOnce(&str) -> T,
) -> Option<T> {
    if !nodes.is_empty() {
        let text = node_text(nodes[0], None);
        Some(wrapper(&text))
    } else {
        None
    }
}

/// Port of `ichiran/kanji:load-readings` (`kanji.lisp:116`).
///
/// Walks the `<reading>` element list of a kanjidic2 character,
/// filters down to the `ja_on` and `ja_kun` entries, normalises each
/// to hiragana, strips the prefix/suffix `-` markers, splits off the
/// okurigana portion at the `.` separator, deduplicates by reading
/// text (an entry seen with both `ja_on` and `ja_kun` is recorded
/// once as `ja_onkun`, with okurigana forms accumulated and the
/// suffix/prefix flags OR-ed in), and inserts one `reading` row per
/// distinct reading plus one `okurigana` row per accumulated
/// okurigana form. Reading rows are inserted in kanjidic2 `<reading>`
/// order so each `reading.id` is assigned deterministically.
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
        .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
        .await?;
        // kanji.lisp:149-151 (loop with rid = (id robj) for of in okuri do …)
        for of in &info.okuri {
            sqlx::query("INSERT INTO okurigana (reading_id, text) VALUES ($1, $2)")
                .bind(robj.id)
                .bind(of)
                .execute(ctx.pool.as_ref().expect("postgres pool"))
                .await?;
        }
    }
    Ok(())
}

/// Port of `ichiran/kanji:load-kanji` (`kanji.lisp:155`).
///
/// Parses one kanjidic2 `<character>` XML payload, inserts the `kanji`
/// row and its associated `reading`, `nanori`-style reading
/// (`type = "ja_na"`), and English `meaning` rows. The `<reading>`
/// list is handed off to [`super::load_readings`] for dedup +
/// okurigana extraction; `<nanori>` and `<meaning>` are inserted in
/// the order they appear in the source.
pub async fn load_kanji(ctx: &KaniranContext, content: &str) -> Result<(), sqlx::Error> {
    // kanji.lisp:156 (parsed (cxml:parse content (cxml-dom:make-dom-builder)))
    let parsed = Document::parse(content).expect("load_kanji: malformed character XML");
    let root = parsed.root_element();
    let descendants: Vec<Node<'_, '_>> = root.descendants().filter(|n| n.is_element()).collect();
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
        if r#type == "classical" && radical_c.is_none() {
            radical_c = Some(radical);
        } else if r#type == "nelson_c" && radical_n.is_none() {
            radical_n = Some(radical);
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
    .fetch_one(ctx.pool.as_ref().expect("postgres pool"))
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
        .execute(ctx.pool.as_ref().expect("postgres pool"))
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
                .execute(ctx.pool.as_ref().expect("postgres pool"))
                .await?;
        }
    }
    Ok(())
}

/// Port of `ichiran/kanji:load-kanjidic` (`kanji.lisp:187`).
///
/// Rebuilds the four kanjidic tables (`kanji`, `reading`, `okurigana`,
/// `meaning`) from a kanjidic2 XML dump. Drops & recreates the schema
/// via [`super::loaders::init_tables`], iterates every `<character>`
/// child of the `<kanjidic2>` root, serializes each `<character>`
/// element back into an XML fragment, hands it to
/// [`super::loaders::load_kanji`], emits a progress line every
/// 500 entries, and finishes with `ANALYZE`.
pub async fn load_kanjidic(ctx: &KaniranContext, path: &Path) -> Result<(), sqlx::Error> {
    // kanji.lisp:188 (init-tables)
    init_tables(ctx).await?;
    // kanji.lisp:189-191 (klacks:with-open-source (source (cxml:make-source path)) (klacks:find-element source "kanjidic2") …)
    let source =
        std::fs::read_to_string(path).expect("load_kanjidic: failed to read kanjidic2 source");
    let parsed = Document::parse_with_options(
        &source,
        ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .expect("load_kanjidic: malformed kanjidic2 XML");
    let kanjidic2 = parsed
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("kanjidic2"))
        .expect("load_kanjidic: missing kanjidic2 root element");
    // kanji.lisp:192-197 (loop for cnt from 1 while (klacks:find-element source "character") …)
    let mut cnt: i32 = 1;
    for character_node in kanjidic2
        .children()
        .filter(|n| n.is_element() && n.has_tag_name("character"))
    {
        let content = serialize_character(character_node);
        load_kanji(ctx, &content).await?;
        if cnt % 500 == 0 {
            println!("{cnt} entries loaded");
        }
        cnt += 1;
    }
    // kanji.lisp:197 (finally (query "ANALYZE") (format t "~a entries total~%" cnt))
    sqlx::query("ANALYZE").execute(ctx.pool.as_ref().expect("postgres pool")).await?;
    println!("{cnt} entries total");
    Ok(())
}

/// Walks a `<character>` subtree and produces an XML string suitable
/// for [`super::loaders::load_kanji`] — mirrors
/// `klacks:serialize-element source (cxml:make-string-sink)`.
fn serialize_character(node: Node<'_, '_>) -> String {
    let mut out = String::new();
    write_node(node, &mut out);
    out
}

fn write_node(node: Node<'_, '_>, out: &mut String) {
    match node.node_type() {
        NodeType::Element => {
            let name = node.tag_name().name();
            out.push('<');
            out.push_str(name);
            for attr in node.attributes() {
                out.push(' ');
                out.push_str(attr.name());
                out.push_str("=\"");
                write_escaped_attr(attr.value(), out);
                out.push('"');
            }
            if node.first_child().is_none() {
                out.push_str("/>");
                return;
            }
            out.push('>');
            for child in node.children() {
                write_node(child, out);
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        NodeType::Text => {
            if let Some(text) = node.text() {
                write_escaped_text(text, out);
            }
        }
        _ => {}
    }
}

fn write_escaped_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            _ => out.push(c),
        }
    }
}

fn write_escaped_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Port of `ichiran/kanji:load-kanji-stats` (`kanji.lisp:333`).
///
/// Recomputes the per-kanji and per-reading frequency statistics over
/// every grade-1..8 kanji from the live corpus: zeroes the existing
/// counters, walks each kanji through [`kanji_word_stats`], and writes
/// the resulting `stat_common` / `stat_irregular` totals back. Ends with
/// an `ANALYZE`. Was a `pub async fn` in `kaniran-core`'s `kanji/stats.rs`
/// before the load-time half split into this crate; the now-sync
/// `kanji_word_stats` lookup is called without `.await`.
pub async fn load_kanji_stats(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    // kanji.lisp:334 (query (:update 'kanji :set 'stat-common 0 'stat-irregular 0))
    sqlx::query("UPDATE kanji SET stat_common = 0, stat_irregular = 0")
        .execute(ctx.pool.as_ref().expect("postgres pool"))
        .await?;
    // kanji.lisp:335 (query (:update 'reading :set 'stat-common 0))
    sqlx::query("UPDATE reading SET stat_common = 0")
        .execute(ctx.pool.as_ref().expect("postgres pool"))
        .await?;
    // kanji.lisp:336 (select-dao 'kanji (:<= 'grade 8))
    let kanji_rows: Vec<Kanji> = sqlx::query_as("SELECT * FROM kanji WHERE grade <= 8")
        .fetch_all(ctx.pool.as_ref().expect("postgres pool"))
        .await?;
    // kanji.lisp:336-347 (loop for kanji in … …)
    // `cnt` starts at 1 and steps after each body (CL loop `for cnt from 1`
    // post-increments), so the `finally` clause sees `cnt = N+1`.
    let mut cnt: i32 = 1;
    for kanji in &kanji_rows {
        // kanji.lisp:338 ((reading-stats irregular total) = (multiple-value-list (kanji-word-stats (text kanji))))
        let (reading_stats, irregular, total) =
            kanji_word_stats(ctx, &kanji.text).map_err(crate::kani_db_to_sqlx)?;
        // kanji.lisp:339 (readings = (select-dao 'reading (:= 'kanji-id (id kanji))))
        let readings: Vec<Reading> = sqlx::query_as("SELECT * FROM reading WHERE kanji_id = $1")
            .bind(kanji.id)
            .fetch_all(ctx.pool.as_ref().expect("postgres pool"))
            .await?;
        // kanji.lisp:340-342 (setf (stat-common kanji) total (stat-irregular kanji) irregular) (update-dao kanji)
        sqlx::query("UPDATE kanji SET stat_common = $1, stat_irregular = $2 WHERE id = $3")
            .bind(total as i32)
            .bind(irregular)
            .bind(kanji.id)
            .execute(ctx.pool.as_ref().expect("postgres pool"))
            .await?;
        // kanji.lisp:343-345 (loop for ((rtext rtype) . rcount) in reading-stats …)
        for ((rtext, rtype), rcount) in &reading_stats {
            // kanji.lisp:344 (find-if (lambda (r) (and (equal (text r) rtext) (equal (reading-type r) rtype))) readings)
            let reading = readings
                .iter()
                .find(|r| r.text == *rtext && r.reading_type == *rtype);
            if let Some(reading) = reading {
                sqlx::query("UPDATE reading SET stat_common = $1 WHERE id = $2")
                    .bind(*rcount)
                    .bind(reading.id)
                    .execute(ctx.pool.as_ref().expect("postgres pool"))
                    .await?;
            }
        }
        // kanji.lisp:346 (if (zerop (mod cnt 100)) do (format t "~a kanji processed~%" cnt))
        if cnt % 100 == 0 {
            println!("{cnt} kanji processed");
        }
        cnt += 1;
    }
    // kanji.lisp:347 (finally (query "ANALYZE") (format t "~a kanji total~%" cnt))
    sqlx::query("ANALYZE")
        .execute(ctx.pool.as_ref().expect("postgres pool"))
        .await?;
    println!("{cnt} kanji total");
    Ok(())
}

#[cfg(test)]
mod tests;
