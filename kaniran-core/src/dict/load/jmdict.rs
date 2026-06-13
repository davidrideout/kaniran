use crate::characters::text::join;
use crate::conn::kani_context::KaniranContext;
use crate::custom::load::{load_custom_data, LoadCustomDataError};
use crate::dict::dao::Entry;
use crate::dict::errata::add_errata;
use crate::dict::readings::{find_word, FindWordRows};
use crate::dict::senses::{get_senses_raw, RawSense};
use crate::dict::load::conj_rules::POS_WITH_CONJ_RULES;
use crate::dict::load::conjugate::{
    conjugate_entry_outer, load_conjugations, load_secondary_conjugations,
};
use crate::dict::load::readings::{insert_readings, ReadingTable};
use crate::dict::dao::recalc_entry_stats_all;
use fancy_regex::{Captures, Regex};
use roxmltree::{Document, Node, NodeType, ParsingOptions};
use std::path::Path;
use std::sync::OnceLock;

/// Port of `ichiran/dict:node-text` (`dict-load.lisp:18`).
///
/// Concatenates text content of a DOM subtree
pub fn node_text<'a, 'input>(
    node: Node<'a, 'input>,
    test: Option<&dyn Fn(Node<'a, 'input>) -> bool>,
) -> String {
    let mut values: Vec<String> = Vec::new();
    if test.map_or(true, |t| t(node)) {
        for child in node.children() {
            match child.node_type() {
                NodeType::Element => values.push(node_text(child, test)),
                NodeType::Text => {
                    if let Some(value) = child.text() {
                        values.push(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    values.concat()
}

/// Port of `ichiran/dict:insert-sense-traits` (`dict-load.lisp:66`).
///
/// For every descendant of `sense_node` whose element name matches
/// `tag`, INSERTs a `sense_prop` row carrying that descendant's text
/// content.
pub async fn insert_sense_traits(
    ctx: &KaniranContext,
    sense_node: Node<'_, '_>,
    tag: &str,
    sense_id: i32,
    seq: i32,
) -> Result<(), sqlx::Error> {
    for (ord, node) in sense_node
        .descendants()
        .filter(|n| *n != sense_node && n.is_element() && n.has_tag_name(tag))
        .enumerate()
    {
        let text = node_text(node, None);
        sqlx::query(
            "INSERT INTO sense_prop (tag, sense_id, text, ord, seq) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(tag)
        .bind(sense_id)
        .bind(&text)
        .bind(ord as i32)
        .bind(seq)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:insert-senses` (`dict-load.lisp:71`).
///
/// For each element of `node_list` (the JMdict `<sense>` nodes of one
/// entry), INSERTs a `sense` row, then INSERTs each child `<gloss>` as
/// a `gloss` row.
const SENSE_PROP_TAGS: &[&str] = &["pos", "misc", "dial", "field", "s_inf", "stagk", "stagr"];

pub async fn insert_senses(
    ctx: &KaniranContext,
    node_list: &[Node<'_, '_>],
    seq: i32,
) -> Result<(), sqlx::Error> {
    for (ord, node) in node_list.iter().enumerate() {
        let sense_id: i32 =
            sqlx::query_scalar("INSERT INTO sense (seq, ord) VALUES ($1, $2) RETURNING id")
                .bind(seq)
                .bind(ord as i32)
                .fetch_one(&ctx.pool)
                .await?;
        for (gord, gloss_node) in node
            .descendants()
            .filter(|n| *n != *node && n.is_element() && n.has_tag_name("gloss"))
            .enumerate()
        {
            let text = node_text(gloss_node, None);
            sqlx::query("INSERT INTO gloss (sense_id, text, ord) VALUES ($1, $2, $3)")
                .bind(sense_id)
                .bind(&text)
                .bind(gord as i32)
                .execute(&ctx.pool)
                .await?;
        }
        for tag in SENSE_PROP_TAGS {
            insert_sense_traits(ctx, *node, tag, sense_id, seq).await?;
        }
    }
    Ok(())
}

/// Port of `ichiran/dict:sense-exists-p` (`dict-load.lisp:82`).
///
/// Returns true when some sense in `senses` has the same parts of
/// speech and glosses as the candidate. A sense with no parts of
/// speech of its own inherits them from the most recent earlier sense
/// that had them.
pub fn sense_exists_p(senses: &[RawSense], positions: &[String], glosses: &[String]) -> bool {
    let glosses_str = join("; ", glosses);
    let mut rpos: Option<&[String]> = None;
    let mut first = true;
    for sense in senses {
        let pos: Option<&[String]> = sense
            .props
            .iter()
            .find(|(tag, _)| tag == "pos")
            .map(|(_, vals)| vals.as_slice());
        // dict-load.lisp:88 (for rpos = pos then (or pos rpos))
        rpos = if first { pos } else { pos.or(rpos) };
        first = false;
        let pos_match = match rpos {
            Some(rp) => rp == positions,
            None => positions.is_empty(),
        };
        if pos_match && glosses_str == sense.gloss {
            return true;
        }
    }
    false
}

/// Port of `ichiran/dict:add-new-sense` (`dict-load.lisp:91`).
///
/// Adds a sense to the entry. Inserts the sense row, its glosses, and
/// the pos sense-props (only when they differ from the entry's last
/// seen pos). Returns `None` if a matching sense already exists.
pub async fn add_new_sense(
    ctx: &KaniranContext,
    seq: i32,
    positions: &[String],
    glosses: &[String],
) -> Result<Option<(i32, i32)>, sqlx::Error> {
    let senses = get_senses_raw(ctx, seq).await?;
    if sense_exists_p(&senses, positions, glosses) {
        return Ok(None);
    }
    let last_sense = senses.last().expect("add_new_sense: entry has no senses");
    let ord = last_sense.ord + 1;
    // dict-load.lisp:98-101 (loop for s in (reverse senses) ... thereis pos)
    let last_pos: Option<&[String]> = senses.iter().rev().find_map(|s| {
        s.props
            .iter()
            .find(|(tag, _)| tag == "pos")
            .map(|(_, vals)| vals.as_slice())
    });
    let sense_id: i32 =
        sqlx::query_scalar("INSERT INTO sense (seq, ord) VALUES ($1, $2) RETURNING id")
            .bind(seq)
            .bind(ord)
            .fetch_one(&ctx.pool)
            .await?;
    for (gord, gloss) in glosses.iter().enumerate() {
        sqlx::query("INSERT INTO gloss (sense_id, text, ord) VALUES ($1, $2, $3)")
            .bind(sense_id)
            .bind(gloss)
            .bind(gord as i32)
            .execute(&ctx.pool)
            .await?;
    }
    let last_pos_matches = match last_pos {
        Some(lp) => lp == positions,
        None => positions.is_empty(),
    };
    if !last_pos_matches {
        for (sord, pos) in positions.iter().enumerate() {
            sqlx::query(
                "INSERT INTO sense_prop (sense_id, tag, text, ord, seq) \
                 VALUES ($1, 'pos', $2, $3, $4)",
            )
            .bind(sense_id)
            .bind(pos)
            .bind(sord as i32)
            .bind(seq)
            .execute(&ctx.pool)
            .await?;
        }
    }
    Ok(Some((sense_id, ord)))
}

/// Port of `ichiran/dict:init-tables` (`dict-load.lisp:7`).
///
/// Empties the entry-package tables so JMdict data can be reloaded
/// into a clean schema.
pub const TABLE_NAMES: &[&str] = &[
    "entry",
    "kanji_text",
    "kana_text",
    "sense",
    "gloss",
    "sense_prop",
    "conjugation",
    "conj_prop",
    "conj_source_reading",
    "restricted_readings",
];

pub async fn init_tables(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    sqlx::query(
        "TRUNCATE entry, kanji_text, kana_text, sense, gloss, sense_prop, \
         conjugation, conj_prop, conj_source_reading, restricted_readings \
         RESTART IDENTITY CASCADE",
    )
    .execute(&ctx.pool)
    .await?;
    Ok(())
}

/// Port of `ichiran/dict:next-seq` (`dict-load.lisp:112`).
///
/// `MAX(seq) + 1` from `entry`. Panics if the table is empty
pub async fn next_seq(ctx: &KaniranContext) -> Result<i32, sqlx::Error> {
    let max: Option<i32> = sqlx::query_scalar("SELECT MAX(seq) FROM entry")
        .fetch_one(&ctx.pool)
        .await?;
    Ok(max.expect("next_seq: entry table is empty") + 1)
}

/// Port of `ichiran/dict:load-entry` (`dict-load.lisp:115`).
///
/// Parses one JMdict `<entry>` XML element, inserts the `entry` row
/// plus its `kanji_text`, `kana_text`, and `sense` children, and
/// optionally conjugates verbs/adjectives. Returns `Some(seq)` on
/// success, `None` on `:skip` / upstream-text-match early-return.
///
/// `content` is the serialized XML for a single entry (one `<entry>`
/// element, entities already expanded by [`fix_entities`]).
///
/// [`fix_entities`]: crate::dict::fix_entities
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
    let parsed = Document::parse(content).expect("load_entry: malformed entry XML");
    // dict-load.lisp:123-132 (seq (cond ...))
    let seq: i32 = match seq {
        LoadEntrySeq::Str(s) => match &*find_word(ctx, s, false).await? {
            FindWordRows::Kana(rows) => match rows.first() {
                Some(row) => row.seq,
                None => next_seq(ctx).await?,
            },
            FindWordRows::Kanji(rows) => match rows.first() {
                Some(row) => row.seq,
                None => next_seq(ctx).await?,
            },
        },
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
        let entry: Option<Entry> = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
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
            let exists: Option<i32> = sqlx::query_scalar("SELECT seq FROM entry WHERE seq = $1")
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
    insert_readings(
        ctx,
        &kanji_nodes,
        "keb",
        ReadingTable::KanjiText,
        seq,
        "ke_pri",
    )
    .await?;
    insert_readings(
        ctx,
        &kana_nodes,
        "reb",
        ReadingTable::KanaText,
        seq,
        "re_pri",
    )
    .await?;
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

/// Port of `ichiran/dict:fix-entities` (`dict-load.lisp:161`).
fn entity_decl_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"<!ENTITY\s+([A-Za-z_:][A-Za-z0-9._:\-]*)\s+(?:"([^"]*)"|'([^']*)')\s*>"#)
            .expect("fix_entities regex")
    })
}

pub fn fix_entities(source: &str) -> String {
    entity_decl_regex()
        .replace_all(source, |caps: &Captures<'_>| {
            let name = caps.get(1).expect("entity name").as_str();
            match name {
                "lt" | "gt" | "amp" | "apos" | "quot" => {
                    caps.get(0).expect("match").as_str().to_string()
                }
                _ => {
                    let quote_char = if caps.get(2).is_some() { '"' } else { '\'' };
                    format!("<!ENTITY {name} {quote_char}{name}{quote_char}>")
                }
            }
        })
        .into_owned()
}

/// Port of `ichiran/dict:load-jmdict` (`dict-load.lisp:170`).
///
/// Rebuilds the entry-package tables from a JMdict XML dump: clears
/// the schema via [`crate::dict::init_tables`], iterates every `<entry>` in
/// the source, hands each to [`crate::dict::load_entry`], and (when
/// requested) chains [`crate::dict::load_extras`] for the conjugation /
/// errata / custom-data pass.
pub async fn load_jmdict(
    ctx: &KaniranContext,
    path: &Path,
    load_extras_p: bool,
) -> Result<(), LoadCustomDataError> {
    init_tables(ctx).await?;
    let source = std::fs::read_to_string(path)?;
    let fixed = fix_entities(&source);
    let parsed = Document::parse_with_options(
        &fixed,
        ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .expect("load_jmdict: malformed JMdict XML");
    // dict-load.lisp:174 (klacks:find-element source "JMdict")
    let jmdict = parsed
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("JMdict"))
        .expect("load_jmdict: missing JMdict root element");
    // dict-load.lisp:176-182 (loop ... while (klacks:find-element source "entry") ...)
    let mut cnt: i32 = 0;
    for entry_node in jmdict
        .children()
        .filter(|n| n.is_element() && n.has_tag_name("entry"))
    {
        cnt += 1;
        let content = serialize_entry(entry_node);
        // Upstream `(load-entry content)` passes no :conjugate-p (defaults nil):
        // conjugation runs later in one pass via load-extras → load-conjugations.
        // Conjugating per-entry here would call next-seq (MAX+1) for synthetic
        // forms that then collide with later JMdict ent_seqs.
        load_entry(
            ctx,
            &content,
            LoadEntryIfExists::None,
            None,
            LoadEntrySeq::None,
            false,
        )
        .await?;
        if cnt % 1000 == 0 {
            println!("{cnt} entries loaded");
        }
    }
    recalc_entry_stats_all(ctx).await?;
    sqlx::query("ANALYZE").execute(&ctx.pool).await?;
    println!("{cnt} entries total");
    if load_extras_p {
        load_extras(ctx).await?;
    }
    Ok(())
}

/// Walks an `<entry>` subtree and produces an XML string matching the
/// byte shape `(klacks:serialize-element source (cxml:make-string-sink))`
/// emits in upstream `load-jmdict`. Every entity reference has already
/// been resolved to its short name by [`fix_entities`], so the output
/// is standalone XML with no DTD attached.
///
/// Two cxml behaviors that the raw walk needs to reproduce:
/// 1. Prepend the XML prolog `<?xml version="1.0" encoding="UTF-8"?>\n`
///    that `make-string-sink` adds in front of the element body. The
///    body ends at `</entry>` with no trailing newline — verified
///    against `ichiran_260118.entry.content` (`octet_length` matches
///    the no-newline length exactly).
/// 2. Inject DTD-default attributes that the JMdict DTD declares on
///    `<gloss>` / `<lsource>` / `<ex_sent>` (`xml:lang CDATA "eng"`).
///    cxml's DOM build applies the DTD defaults at parse time, then
///    the sink emits them back out alongside source-specified attrs;
///    roxmltree does not, so the port injects them explicitly.
fn serialize_entry(node: Node<'_, '_>) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write_node(node, &mut out);
    out
}

/// DTD-default attribute decl: `<!ATTLIST $0 $1 CDATA "$2">`. cxml emits
/// the default as the first attribute on the element when the source
/// XML didn't supply one explicitly — putting it before any
/// source-specified attrs (e.g. `<gloss xml:lang="eng" g_type="expl">`).
const DTD_DEFAULT_ATTRS: &[(&str, &str, &str)] = &[
    ("gloss", "xml:lang", "eng"),
    ("lsource", "xml:lang", "eng"),
    ("ex_sent", "xml:lang", "eng"),
];

// The W3C XML namespace — `xml:lang`, `xml:space` etc. roxmltree exposes
// these with namespace=this URI and a bare local name; the original
// source uses the `xml:` prefix and the serializer has to reconstruct it.
const XML_NAMESPACE_URI: &str = "http://www.w3.org/XML/1998/namespace";

fn write_node(node: Node<'_, '_>, out: &mut String) {
    match node.node_type() {
        NodeType::Element => {
            let name = node.tag_name().name();
            out.push('<');
            out.push_str(name);
            for (elem, attr_name, default_val) in DTD_DEFAULT_ATTRS {
                if *elem == name && !has_attr(node, attr_name) {
                    out.push(' ');
                    out.push_str(attr_name);
                    out.push_str("=\"");
                    write_escaped_attr(default_val, out);
                    out.push('"');
                }
            }
            for attr in node.attributes() {
                out.push(' ');
                if attr.namespace() == Some(XML_NAMESPACE_URI) {
                    out.push_str("xml:");
                }
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

/// True if `node` already declares an attribute matching `attr_name`,
/// where `attr_name` may be a namespaced `xml:foo` form. Used to skip
/// DTD-default injection when the source supplied the attribute itself.
fn has_attr(node: Node<'_, '_>, attr_name: &str) -> bool {
    if let Some(local) = attr_name.strip_prefix("xml:") {
        node.attributes()
            .any(|a| a.namespace() == Some(XML_NAMESPACE_URI) && a.name() == local)
    } else {
        node.attributes()
            .any(|a| a.name() == attr_name && a.namespace().is_none())
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

/// Port of `ichiran/dict:load-extras` (`dict-load.lisp:185`).
///
/// Build-time pipeline that rehydrates everything downstream of the
/// raw JMdict load: conjugations, secondary conjugations, custom data,
/// errata, and a final `entry` row-count refresh.
pub async fn load_extras(ctx: &KaniranContext) -> Result<(), LoadCustomDataError> {
    println!("Loading conjugations...");
    load_conjugations(ctx).await?;
    println!("Loading secondary conjugations...");
    load_secondary_conjugations(ctx, None).await?;
    println!("Loading custom data...");
    // dict-load.lisp:191 (ichiran/custom:load-custom-data nil t)
    load_custom_data(ctx, &[], true).await?;
    add_errata(ctx).await?;
    recalc_entry_stats_all(ctx).await?;
    sqlx::query("ANALYZE").execute(&ctx.pool).await?;
    Ok(())
}

/// Port of `ichiran/dict:drop-extras` (`dict-load.lisp:196`).
///
/// Wipes the rows added back on top of a raw JMdict load — every
/// conjugation, every conjugation property, every conj-source-reading,
/// and every non-root `entry` row.
pub async fn drop_extras(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM conj_prop")
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM conj_source_reading")
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM conjugation")
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM entry WHERE NOT root_p")
        .execute(&ctx.pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
