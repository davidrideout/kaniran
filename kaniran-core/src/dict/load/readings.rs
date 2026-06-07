use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::KanaText;
use crate::dict::dao::KanjiText;
use crate::dict::load::jmdict::node_text;
use roxmltree::Node;

/// Port of `ichiran/dict:set-reading` (gf — `dict-load.lisp:487`)
/// and the two methods on `kanji-text` (`dict-load.lisp:490`) and
/// `kana-text` (`dict-load.lisp:507`).
///
/// Picks the best cross-reading for a kanji/kana row and writes it
/// back to the row's `best_kana` / `best_kanji` column. Restricted
/// readings (`re_restr` JMdict tags) gate the candidate list.
pub enum SetReadingObj<'a> {
    Kanji(&'a mut KanjiText),
    Kana(&'a mut KanaText),
}

pub async fn set_reading(ctx: &KaniranContext, obj: SetReadingObj<'_>) -> Result<(), sqlx::Error> {
    match obj {
        SetReadingObj::Kanji(obj) => set_reading_kanji(ctx, obj).await,
        SetReadingObj::Kana(obj) => set_reading_kana(ctx, obj).await,
    }
}

async fn set_reading_kanji(ctx: &KaniranContext, obj: &mut KanjiText) -> Result<(), sqlx::Error> {
    let seq = obj.seq;
    let cur_best = obj.best_kana.clone();
    // dict-load.lisp:493 (query (:select 'reading 'text :from 'restricted-readings :where (:= 'seq seq)))
    let restricted: Vec<(String, String)> =
        sqlx::query_as("SELECT reading, text FROM restricted_readings WHERE seq = $1")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    // dict-load.lisp:494 (loop for reading in (select-dao 'kana-text (:= 'seq seq) 'ord))
    let readings: Vec<KanaText> =
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 ORDER BY ord")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    for reading in &readings {
        let rtext = &reading.text;
        // dict-load.lisp:496 (for restr = (loop for (rt kt) in restricted when (equal rtext rt) collect kt))
        let restr: Vec<&String> = restricted
            .iter()
            .filter_map(|(rt, kt)| if rt == rtext { Some(kt) } else { None })
            .collect();
        // dict-load.lisp:497-499 (when (and (not (nokanji reading)) (or (not restr) (member (text obj) restr :test 'equal))))
        if !reading.nokanji && (restr.is_empty() || restr.iter().any(|kt| *kt == &obj.text)) {
            // dict-load.lisp:500-501 (unless (equal cur-best (text reading)) (setf (best-kana obj) (text reading)) (update-dao obj))
            if cur_best.as_deref() != Some(rtext.as_str()) {
                sqlx::query("UPDATE kanji_text SET best_kana = $1 WHERE id = $2")
                    .bind(rtext)
                    .bind(obj.id)
                    .execute(&ctx.pool)
                    .await?;
                obj.best_kana = Some(rtext.clone());
            }
            // dict-load.lisp:502 (return-from set-reading)
            return Ok(());
        }
    }
    // dict-load.lisp:503-504 (unless (equal cur-best :null) (setf (best-kana obj) :null) (update-dao obj))
    if cur_best.is_some() {
        sqlx::query("UPDATE kanji_text SET best_kana = NULL WHERE id = $1")
            .bind(obj.id)
            .execute(&ctx.pool)
            .await?;
        obj.best_kana = None;
    }
    Ok(())
}

async fn set_reading_kana(ctx: &KaniranContext, obj: &mut KanaText) -> Result<(), sqlx::Error> {
    // dict-load.lisp:508-509 (when (nokanji obj) (return-from set-reading))
    if obj.nokanji {
        return Ok(());
    }
    let seq = obj.seq;
    let cur_best = obj.best_kanji.clone();
    let rtext = obj.text.clone();
    // dict-load.lisp:513-516 (query (:select 'text :from 'restricted-readings
    //   :where (:and (:= 'seq seq) (:= 'reading rtext))) :column)
    let restricted: Vec<String> =
        sqlx::query_scalar("SELECT text FROM restricted_readings WHERE seq = $1 AND reading = $2")
            .bind(seq)
            .bind(&rtext)
            .fetch_all(&ctx.pool)
            .await?;
    // dict-load.lisp:517-521 (if restricted
    //   (select-dao 'kanji-text (:and (:= 'seq seq) (:in 'text (:set restricted))) 'ord)
    //   (select-dao 'kanji-text (:= 'seq seq) 'ord))
    let kanji_list: Vec<KanjiText> = if !restricted.is_empty() {
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = ANY($2) ORDER BY ord")
            .bind(seq)
            .bind(&restricted)
            .fetch_all(&ctx.pool)
            .await?
    } else {
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 ORDER BY ord")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?
    };
    // dict-load.lisp:522-530 (cond (kanji-list ...) (t ...))
    if let Some(first_kt) = kanji_list.first() {
        let ktext = &first_kt.text;
        if cur_best.as_deref() != Some(ktext.as_str()) {
            sqlx::query("UPDATE kana_text SET best_kanji = $1 WHERE id = $2")
                .bind(ktext)
                .bind(obj.id)
                .execute(&ctx.pool)
                .await?;
            obj.best_kanji = Some(ktext.clone());
        }
    } else if cur_best.is_some() {
        sqlx::query("UPDATE kana_text SET best_kanji = NULL WHERE id = $1")
            .bind(obj.id)
            .execute(&ctx.pool)
            .await?;
        obj.best_kanji = None;
    }
    Ok(())
}

/// Port of `ichiran/dict:insert-readings` (`dict-load.lisp:32`).
///
/// Inserts the kanji or kana readings for one entry. Skips readings
/// tagged `re_inf=ok`, records `re_restr` restrictions, then updates
/// the parent entry's reading count and `primary_nokanji` flag.
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

/// Port of `ichiran/dict:load-best-readings` (`dict-load.lisp:532`).
///
/// Refreshes the cached `best_kana` / `best_kanji` cross-references on
/// every root entry's kanji/kana rows by streaming them through
/// [`set_reading`]. With `reset`, every row's cached column is first
/// NULLed so the second pass recomputes from scratch.
pub async fn load_best_readings(ctx: &KaniranContext, reset: bool) -> Result<(), sqlx::Error> {
    // dict-load.lisp:534-536 (when reset ...)
    if reset {
        sqlx::query("UPDATE kanji_text SET best_kana = NULL")
            .execute(&ctx.pool)
            .await?;
        sqlx::query("UPDATE kana_text SET best_kanji = NULL")
            .execute(&ctx.pool)
            .await?;
    }
    // dict-load.lisp:537-542 (loop for kanji in (query-dao 'kanji-text ...))
    let kanji_rows: Vec<KanjiText> = sqlx::query_as(
        "SELECT kt.* FROM kanji_text kt, entry \
         WHERE kt.seq = entry.seq AND kt.best_kana IS NULL AND entry.root_p",
    )
    .fetch_all(&ctx.pool)
    .await?;
    for mut kanji in kanji_rows {
        set_reading(ctx, SetReadingObj::Kanji(&mut kanji)).await?;
    }
    // dict-load.lisp:543-548 (loop for kana in (query-dao 'kana-text ...))
    let kana_rows: Vec<KanaText> = sqlx::query_as(
        "SELECT kt.* FROM kana_text kt, entry \
         WHERE kt.seq = entry.seq AND kt.best_kanji IS NULL AND entry.root_p",
    )
    .fetch_all(&ctx.pool)
    .await?;
    for mut kana in kana_rows {
        set_reading(ctx, SetReadingObj::Kana(&mut kana)).await?;
    }
    Ok(())
}
