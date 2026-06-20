//! Optional binary (feature `rkyv-dump`): dumps the runtime dictionary
//! tables of a kaniran Postgres database into one rkyv archive
//! ([`KaniSnapshot`]). Every table is fetched `ORDER BY ctid` so the
//! archive preserves physical row order — the order Postgres returns
//! rows for the seam's `ORDER BY`-less queries — letting the in-memory
//! backend reproduce those orderings exactly.
//!
//! ```sh
//! cargo run --release -p kaniran-cli --features rkyv-dump \
//!     --bin kaniran-rkyv-dumper -- --out kaniran_snapshot.rkyv
//! ```
//!
//! Peak memory is roughly twice the archive size (the row vectors plus
//! the serialization buffer); the on-disk table sizes printed at start
//! give the magnitude before any heavy work begins.

use clap::Parser;
use futures_util::TryStreamExt;
use kaniran_core::conn::get_ichiran_connection_env::get_ichiran_connection_env;
use std::collections::{HashMap, HashSet};
use kaniran_core::conn::kani_snapshot::{
    ArchivedKaniSnapshot, KaniKanaTextRow, KaniKanjiTextRow, KaniSnapshot, KaniSnapshotMeta,
    KANI_SNAPSHOT_FORMAT_VERSION,
};
use kaniran_core::dict::dao::{
    ConjProp, ConjSourceReading, Conjugation, Entry, Gloss, RestrictedReadings, Sense, SenseProp,
};
use kaniran_core::kanji::dao::{Kanji, Meaning, Okurigana, Reading};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::PgPool;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const TABLE_NAMES: [&str; 14] = [
    "entry",
    "kanji_text",
    "kana_text",
    "conjugation",
    "conj_prop",
    "conj_source_reading",
    "sense",
    "gloss",
    "sense_prop",
    "restricted_readings",
    "kanji",
    "reading",
    "okurigana",
    "meaning",
];

#[derive(Parser)]
#[command(
    name = "kaniran-rkyv-dumper",
    about = "Dump the runtime dictionary tables into a kaniran rkyv snapshot archive"
)]
struct Args {
    /// Output archive path.
    #[arg(long, default_value = "kaniran_snapshot.rkyv")]
    out: PathBuf,

    /// Postgres URL; falls back to the layered kaniran config
    /// (DATABASE_URL env var).
    #[arg(long)]
    db_url: Option<String>,

    /// Cap the rows fetched per table. Smoke-test runs only — a capped
    /// archive is not a faithful snapshot.
    #[arg(long)]
    limit: Option<i64>,
}

/// `1234567` → `"1,234,567"`.
fn group_digits(value: usize) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

fn rate_per_second(rows: usize, elapsed_seconds: f64) -> String {
    group_digits((rows as f64 / elapsed_seconds.max(0.001)) as usize)
}

/// Stream one table into a row vector in physical (ctid) order,
/// printing progress every 500k rows.
async fn fetch_table<Row>(
    pool: &PgPool,
    table_index: usize,
    table_name: &str,
    limit: Option<i64>,
) -> Result<Vec<Row>, sqlx::Error>
where
    Row: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin,
{
    let started = Instant::now();
    let sql = match limit {
        Some(cap) => format!("SELECT * FROM {table_name} ORDER BY ctid LIMIT {cap}"),
        None => format!("SELECT * FROM {table_name} ORDER BY ctid"),
    };
    let mut rows: Vec<Row> = Vec::new();
    let mut stream = sqlx::query_as::<_, Row>(&sql).fetch(pool);
    while let Some(row) = stream.try_next().await? {
        rows.push(row);
        if rows.len().is_multiple_of(500_000) {
            let elapsed = started.elapsed().as_secs_f64();
            println!(
                "  …{table_name}: {} rows so far, {elapsed:.1}s elapsed, {} rows/s",
                group_digits(rows.len()),
                rate_per_second(rows.len(), elapsed),
            );
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    println!(
        "[{table_index}/{}] {table_name}: {} rows in {elapsed:.1}s ({} rows/s)",
        TABLE_NAMES.len(),
        group_digits(rows.len()),
        rate_per_second(rows.len(), elapsed),
    );
    Ok(rows)
}

/// The tag set the runtime senses query asks for — must match `TAGS` in
/// `kaniran-core/src/dict/senses.rs` (`get_senses_raw`).
const SENSES_QUERY_TAGS: &[&str] = &["pos", "s_inf", "stagk", "stagr", "field"];

/// Pin row order for the senses query where its ORDER BY is ambiguous.
///
/// The runtime senses query sorts by (sense.ord, tag, sense_prop.ord),
/// but ichiran's errata inserts extra pos rows with ord = 0, so a few
/// groups carry duplicate ords and Postgres breaks the tie with an
/// unstable sort that a stable sort over the archive cannot reproduce.
/// Re-run the exact runtime query for the affected seqs and renumber
/// the archived ords to the order Postgres actually returned, making
/// the sort key unique.
async fn pin_ambiguous_sense_prop_order(
    pool: &PgPool,
    senses: &[Sense],
    sense_props: &mut [SenseProp],
) -> Result<(), sqlx::Error> {
    let started = Instant::now();

    // Groups (sense_id, tag) holding duplicate ord values, restricted to
    // the tag set the runtime query asks for.
    let mut ords_seen: HashMap<(i32, usize), HashSet<i32>> = HashMap::new();
    let mut tied_groups: HashSet<(i32, usize)> = HashSet::new();
    for prop in sense_props.iter() {
        let Some(tag_index) = SENSES_QUERY_TAGS.iter().position(|tag| *tag == prop.tag) else {
            continue;
        };
        let group_key = (prop.sense_id, tag_index);
        if !ords_seen.entry(group_key).or_default().insert(prop.ord) {
            tied_groups.insert(group_key);
        }
    }
    if tied_groups.is_empty() {
        println!("sense_prop order pinning: no ambiguous groups found");
        return Ok(());
    }

    let mut affected_seqs: Vec<i32> = sense_props
        .iter()
        .filter(|prop| {
            SENSES_QUERY_TAGS
                .iter()
                .position(|tag| *tag == prop.tag)
                .is_some_and(|tag_index| tied_groups.contains(&(prop.sense_id, tag_index)))
        })
        .map(|prop| prop.seq)
        .collect();
    affected_seqs.sort_unstable();
    affected_seqs.dedup();

    let affected_seq_set: HashSet<i32> = affected_seqs.iter().copied().collect();
    let mut sense_id_by_seq_ord: HashMap<(i32, i32), i32> = HashMap::new();
    for sense in senses {
        if affected_seq_set.contains(&sense.seq) {
            sense_id_by_seq_ord.insert((sense.seq, sense.ord), sense.id);
        }
    }

    // Observed order, captured with the same SQL the runtime backend
    // sends (KaniPostgresBackend::sense_prop_rows_tagged).
    let mut new_ord_by_text: HashMap<(i32, usize, String), i32> = HashMap::new();
    let mut next_rank: HashMap<(i32, usize), i32> = HashMap::new();
    for &seq in &affected_seqs {
        let observed_rows: Vec<(i32, String, String)> = sqlx::query_as(
            "SELECT sense.ord AS ord, sense_prop.tag AS tag, sense_prop.text AS text \
             FROM sense, sense_prop \
             WHERE sense.seq = $1 \
               AND sense_prop.sense_id = sense.id \
               AND sense_prop.tag = ANY($2) \
             ORDER BY sense.ord, sense_prop.tag, sense_prop.ord",
        )
        .bind(seq)
        .bind(SENSES_QUERY_TAGS)
        .fetch_all(pool)
        .await?;
        for (sense_ord, tag, text) in observed_rows {
            let Some(tag_index) = SENSES_QUERY_TAGS.iter().position(|candidate| *candidate == tag)
            else {
                continue;
            };
            let Some(&sense_id) = sense_id_by_seq_ord.get(&(seq, sense_ord)) else {
                continue;
            };
            if !tied_groups.contains(&(sense_id, tag_index)) {
                continue;
            }
            let rank = next_rank.entry((sense_id, tag_index)).or_insert(0);
            if new_ord_by_text
                .insert((sense_id, tag_index, text), *rank)
                .is_some()
            {
                panic!(
                    "sense_prop group (sense_id {sense_id}, tag {}) repeats a text — \
                     observed order is ambiguous, cannot pin",
                    SENSES_QUERY_TAGS[tag_index],
                );
            }
            *rank += 1;
        }
    }

    let mut renumbered = 0usize;
    for prop in sense_props.iter_mut() {
        let Some(tag_index) = SENSES_QUERY_TAGS.iter().position(|tag| *tag == prop.tag) else {
            continue;
        };
        if !tied_groups.contains(&(prop.sense_id, tag_index)) {
            continue;
        }
        match new_ord_by_text.get(&(prop.sense_id, tag_index, prop.text.clone())) {
            Some(&rank) => {
                prop.ord = rank;
                renumbered += 1;
            }
            None => panic!(
                "sense_prop row (sense_id {}, tag {}, text {}) missing from the \
                 observed-order query",
                prop.sense_id, prop.tag, prop.text,
            ),
        }
    }
    println!(
        "pinned sense_prop order: {renumbered} rows renumbered across {} groups / {} seqs in {:.1}s",
        tied_groups.len(),
        affected_seqs.len(),
        started.elapsed().as_secs_f64(),
    );
    Ok(())
}

/// Database name component of a Postgres URL — the last path segment,
/// query string stripped. Credentials and host stay out of the archive.
fn db_name_of(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let total_started = Instant::now();

    let db_url = match args.db_url {
        Some(url) => url,
        None => get_ichiran_connection_env()?
            .ok_or("database URL is not set — pass --db-url or define DATABASE_URL")?,
    };
    if db_url.starts_with("memory://") {
        return Err(
            "the dumper reads from Postgres; pass --db-url postgres://... or set \
             DATABASE_URL=postgres://... (memory:// is the read-only runtime backend)"
                .into(),
        );
    }
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&db_url)
        .await?;

    // Catalog-only footprint pre-pass, so the operator sees the
    // magnitude before the dump holds everything in memory.
    let table_names: Vec<String> = TABLE_NAMES.iter().map(|name| name.to_string()).collect();
    let on_disk_sizes: Vec<(String, String)> = sqlx::query_as(
        "SELECT relname::text, pg_size_pretty(pg_total_relation_size(oid)) \
         FROM pg_class WHERE relname = ANY($1) AND relkind = 'r' \
         ORDER BY pg_total_relation_size(oid) DESC",
    )
    .bind(&table_names)
    .fetch_all(&pool)
    .await?;
    println!("source: {} ({} tables, on-disk incl. indexes):", db_name_of(&db_url), on_disk_sizes.len());
    for (table_name, size_pretty) in &on_disk_sizes {
        println!("  {table_name}: {size_pretty}");
    }
    if let Some(cap) = args.limit {
        println!("row cap ACTIVE: {cap} rows/table — smoke-test archive, not a faithful snapshot");
    }

    let entries: Vec<Entry> = fetch_table(&pool, 1, "entry", args.limit).await?;
    let kanji_texts: Vec<KaniKanjiTextRow> = fetch_table(&pool, 2, "kanji_text", args.limit).await?;
    let kana_texts: Vec<KaniKanaTextRow> = fetch_table(&pool, 3, "kana_text", args.limit).await?;
    let conjugations: Vec<Conjugation> = fetch_table(&pool, 4, "conjugation", args.limit).await?;
    let conj_props: Vec<ConjProp> = fetch_table(&pool, 5, "conj_prop", args.limit).await?;
    let conj_source_readings: Vec<ConjSourceReading> =
        fetch_table(&pool, 6, "conj_source_reading", args.limit).await?;
    let senses: Vec<Sense> = fetch_table(&pool, 7, "sense", args.limit).await?;
    let glosses: Vec<Gloss> = fetch_table(&pool, 8, "gloss", args.limit).await?;
    let mut sense_props: Vec<SenseProp> = fetch_table(&pool, 9, "sense_prop", args.limit).await?;
    let restricted_readings: Vec<RestrictedReadings> =
        fetch_table(&pool, 10, "restricted_readings", args.limit).await?;
    let kanji: Vec<Kanji> = fetch_table(&pool, 11, "kanji", args.limit).await?;
    let readings: Vec<Reading> = fetch_table(&pool, 12, "reading", args.limit).await?;
    let okurigana: Vec<Okurigana> = fetch_table(&pool, 13, "okurigana", args.limit).await?;
    let meanings: Vec<Meaning> = fetch_table(&pool, 14, "meaning", args.limit).await?;
    if args.limit.is_none() {
        pin_ambiguous_sense_prop_order(&pool, &senses, &mut sense_props).await?;
    } else {
        println!("row cap active — skipping sense_prop order pinning");
    }
    pool.close().await;

    let snapshot = KaniSnapshot {
        meta: KaniSnapshotMeta {
            format_version: KANI_SNAPSHOT_FORMAT_VERSION,
            source_db_name: db_name_of(&db_url),
            dumped_at_unix_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        },
        entries,
        kanji_texts,
        kana_texts,
        conjugations,
        conj_props,
        conj_source_readings,
        senses,
        glosses,
        sense_props,
        restricted_readings,
        kanji,
        readings,
        okurigana,
        meanings,
    };

    let serialize_started = Instant::now();
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&snapshot)?;
    println!(
        "serialized: {} bytes in {:.1}s",
        group_digits(bytes.len()),
        serialize_started.elapsed().as_secs_f64(),
    );

    // Structural validation of the buffer we are about to write, plus
    // a per-table row-count cross-check against the fetched vectors.
    let validate_started = Instant::now();
    let archived = rkyv::access::<ArchivedKaniSnapshot, rkyv::rancor::Error>(&bytes)?;
    let count_checks: [(&str, usize, usize); 14] = [
        ("entry", snapshot.entries.len(), archived.entries.len()),
        ("kanji_text", snapshot.kanji_texts.len(), archived.kanji_texts.len()),
        ("kana_text", snapshot.kana_texts.len(), archived.kana_texts.len()),
        ("conjugation", snapshot.conjugations.len(), archived.conjugations.len()),
        ("conj_prop", snapshot.conj_props.len(), archived.conj_props.len()),
        (
            "conj_source_reading",
            snapshot.conj_source_readings.len(),
            archived.conj_source_readings.len(),
        ),
        ("sense", snapshot.senses.len(), archived.senses.len()),
        ("gloss", snapshot.glosses.len(), archived.glosses.len()),
        ("sense_prop", snapshot.sense_props.len(), archived.sense_props.len()),
        (
            "restricted_readings",
            snapshot.restricted_readings.len(),
            archived.restricted_readings.len(),
        ),
        ("kanji", snapshot.kanji.len(), archived.kanji.len()),
        ("reading", snapshot.readings.len(), archived.readings.len()),
        ("okurigana", snapshot.okurigana.len(), archived.okurigana.len()),
        ("meaning", snapshot.meanings.len(), archived.meanings.len()),
    ];
    let mut total_rows = 0usize;
    for (table_name, source_count, archived_count) in count_checks {
        assert_eq!(
            source_count, archived_count,
            "{table_name}: archived row count diverges from fetched rows",
        );
        total_rows += source_count;
    }
    println!(
        "validated: {} rows across {} tables in {:.1}s",
        group_digits(total_rows),
        TABLE_NAMES.len(),
        validate_started.elapsed().as_secs_f64(),
    );

    std::fs::write(&args.out, bytes.as_slice())?;
    println!(
        "wrote {} ({} bytes, format v{}, source {}) — total {:.1}s",
        args.out.display(),
        group_digits(bytes.len()),
        snapshot.meta.format_version,
        snapshot.meta.source_db_name,
        total_started.elapsed().as_secs_f64(),
    );
    Ok(())
}
