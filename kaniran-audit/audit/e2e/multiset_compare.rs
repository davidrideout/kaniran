//! Multiset table-by-table compare between two DBs. Every row becomes a
//! 64-bit hash of all non-`id`-PK columns (each cast to text in SQL so
//! mixed-type tables share one code path). The multiset of hashes is
//! built per side, then diffed: `matched = sum(min(ref[h], test[h]))`,
//! `missing = sum(max(ref[h] - test[h], 0))`, similarly `extra`.
//!
//! Per table, peak memory is 2× (n_rows × ~24 B) — the HashMap entries
//! are u64 → i64 with HashBrown's ~24 B per entry overhead. For the
//! biggest table (~8.6M rows) that's ~400 MB; everything else is
//! trivial.
//!
//! 64-bit hash: at ~10M rows the expected number of accidental
//! collisions across the two sides is on the order of 10⁻¹¹, so the
//! count is exact in practice. If you need byte-for-byte certainty,
//! widen the hash (two AHasher rounds w/ different keys → 128 bits).
//!
//! Run:
//!   cargo run --release --bin multiset_compare -- \
//!       --ichiran-db postgres:///ichiran_260118 \
//!       --new-db postgres:///ichiran_full_e2e \
//!       [--only entry,gloss,...] \
//!       [--report bookkeeping/e2e/multiset_run.md]

use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use futures::TryStreamExt;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

#[derive(Parser)]
struct Args {
    /// Postgres URL of the reference DB.
    #[arg(long)]
    ichiran_db: String,

    /// Postgres URL of the new DB (kaniran-loaded test target).
    #[arg(long)]
    new_db: String,

    /// Comma-separated whitelist of tables (default: all 14).
    #[arg(long, default_value = "")]
    only: String,

    /// Output report path.
    #[arg(long, default_value = "bookkeeping/e2e/multiset_run.md")]
    report: PathBuf,
}

/// Per-table SELECT. Every seq-bearing column holds either a real
/// JMdict ent_seq (stable — same value across builds) or a locally
/// allocated `MAX(seq)+1` id (fake — shuffles per run). Mixed columns
/// (`entry.seq`, `kanji_text.seq`, `kana_text.seq`, `conjugation.from`)
/// are wrapped in `CASE WHEN root_p THEN seq::text END` so root rows
/// contribute their real seq to the hash and non-root rows contribute
/// NULL. Purely-fake columns (`conjugation.seq`, `conjugation.via`) and
/// the serial `id` PK on most tables are dropped. Always-real seqs
/// (`sense.seq`, `sense_prop.seq`, `restricted_readings.seq`) are kept
/// as-is — those tables only hold root rows.
const TABLE_SELECTS: &[(&str, &str)] = &[
    (
        "entry",
        "SELECT CASE WHEN root_p THEN seq::text END, content, \
         root_p::text, n_kanji::text, n_kana::text, \
         primary_nokanji::text FROM entry",
    ),
    (
        "kanji_text",
        "SELECT CASE WHEN e.root_p THEN k.seq::text END, k.text, \
         k.ord::text, k.common::text, k.common_tags, \
         k.conjugate_p::text, k.nokanji::text, k.best_kana \
         FROM kanji_text k JOIN entry e ON e.seq = k.seq",
    ),
    (
        "kana_text",
        "SELECT CASE WHEN e.root_p THEN k.seq::text END, k.text, \
         k.ord::text, k.common::text, k.common_tags, \
         k.conjugate_p::text, k.nokanji::text, k.best_kanji \
         FROM kana_text k JOIN entry e ON e.seq = k.seq",
    ),
    (
        "sense",
        "SELECT seq::text, ord::text FROM sense",
    ),
    (
        // sense_id is a serial allocated nondeterministically across the
        // two DBs even when the underlying sense is the same. Drop it;
        // (text, ord) is the per-gloss natural-tuple content.
        "gloss",
        "SELECT text, ord::text FROM gloss",
    ),
    (
        // Same — sense_id is dropped; the sense_prop row's natural
        // tuple is (tag, text, ord, seq).
        "sense_prop",
        "SELECT tag, text, ord::text, seq::text FROM sense_prop",
    ),
    (
        "restricted_readings",
        "SELECT seq::text, reading, text FROM restricted_readings",
    ),
    (
        // seq and via are always allocated (fake) — dropped. `from` is
        // the JMdict ent_seq for primary conjugations (root source) and
        // an allocated seq for secondary conjugations (chained from a
        // prior conjugated form). Wrapped in CASE so only the real
        // (root-pointing) values contribute.
        "conjugation",
        "SELECT CASE WHEN ef.root_p THEN c.\"from\"::text END \
         FROM conjugation c JOIN entry ef ON ef.seq = c.\"from\"",
    ),
    (
        // conj_id is a serial; the conjugation it points to is itself
        // identified by (seq, from, via), where seq/via are non-root
        // entry seqs that already shuffle. Drop conj_id; what's left is
        // the per-row content: (conj_type, pos, neg, fml).
        "conj_prop",
        "SELECT conj_type::text, pos, neg::text, fml::text FROM conj_prop",
    ),
    (
        "conj_source_reading",
        "SELECT text, source_text FROM conj_source_reading",
    ),
    (
        "kanji",
        "SELECT text, radical_c::text, radical_n::text, grade::text, \
         strokes::text, freq::text, stat_common::text, \
         stat_irregular::text FROM kanji",
    ),
    (
        // kanji_id is deterministic (kanjidic loads in XML order) but
        // we drop it for consistency with the other FK-serial drops.
        // The (type, text, suffixp, prefixp, stat_common) tuple is the
        // reading's row content.
        "reading",
        "SELECT type, text, suffixp::text, prefixp::text, \
         stat_common::text FROM reading",
    ),
    (
        // reading_id is the nondeterministic serial (HashMap iteration
        // order). Drop it; the row content is just (text).
        "okurigana",
        "SELECT text FROM okurigana",
    ),
    (
        "meaning",
        "SELECT text FROM meaning",
    ),
];

fn hash_row(row: &sqlx::postgres::PgRow, ncols: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for i in 0..ncols {
        // Each column was cast to text in SQL, so we read it as
        // Option<String>. NULL hashes distinctly from "" because
        // Option<&str> includes its discriminant in Hash.
        let v: Option<String> = row.try_get(i).ok().flatten();
        v.hash(&mut hasher);
    }
    hasher.finish()
}

/// Build the ref-side counter only. Test side streams against this
/// counter in [`stream_new_db_against`] so the two HashMaps never coexist.
async fn fetch_multiset(
    pool: &PgPool,
    sql: &str,
    ncols: usize,
) -> Result<HashMap<u64, i64>, sqlx::Error> {
    let mut counter: HashMap<u64, i64> = HashMap::with_capacity(1_000_000);
    let mut stream = sqlx::query(sql).fetch(pool);
    while let Some(row) = stream.try_next().await? {
        let h = hash_row(&row, ncols);
        *counter.entry(h).or_insert(0) += 1;
    }
    Ok(counter)
}

/// Stream the test side; for each row, look up its hash in `ichiran_counter`
/// and decrement (capped at 0). Returns (matched, new_db_rows, extra).
/// After this call, `ichiran_counter`'s remaining positive entries are the
/// ref-only excess (sum them in the caller).
async fn stream_new_db_against(
    pool: &PgPool,
    sql: &str,
    ncols: usize,
    ichiran_counter: &mut HashMap<u64, i64>,
) -> Result<(i64, i64, i64), sqlx::Error> {
    let mut matched = 0i64;
    let mut extra = 0i64;
    let mut new_db_rows = 0i64;
    let mut stream = sqlx::query(sql).fetch(pool);
    while let Some(row) = stream.try_next().await? {
        new_db_rows += 1;
        let h = hash_row(&row, ncols);
        match ichiran_counter.get_mut(&h) {
            Some(c) if *c > 0 => {
                *c -= 1;
                matched += 1;
            }
            _ => {
                extra += 1;
            }
        }
    }
    Ok((matched, new_db_rows, extra))
}

struct TableResult {
    name: &'static str,
    ichiran_rows: i64,
    new_db_rows: i64,
    matched: i64,
    missing: i64,
    extra: i64,
    elapsed_s: f64,
}

async fn compare_one(
    name: &'static str,
    sql: &str,
    ncols: usize,
    ichiran_pool: &PgPool,
    new_db_pool: &PgPool,
    report: &mut BufWriter<File>,
) -> Result<TableResult, sqlx::Error> {
    let t0 = Instant::now();
    // Load ichiran side. Streaming rows are dropped as we hash them,
    // so the only table-sized allocation is the resulting HashMap.
    let mut ichiran_counter = fetch_multiset(ichiran_pool, sql, ncols).await?;
    let ichiran_rows: i64 = ichiran_counter.values().sum();

    // Stream new-db rows against the ichiran counter. After this call
    // ichiran_counter holds only the ichiran-only excess (no new_db
    // HashMap is ever built).
    let (matched, new_db_rows, extra) =
        stream_new_db_against(new_db_pool, sql, ncols, &mut ichiran_counter).await?;
    let missing: i64 = ichiran_counter.values().filter(|&&c| c > 0).sum();
    drop(ichiran_counter); // free before the report write

    let elapsed = t0.elapsed().as_secs_f64();
    let r = TableResult {
        name,
        ichiran_rows,
        new_db_rows,
        matched,
        missing,
        extra,
        elapsed_s: elapsed,
    };
    writeln!(
        report,
        "\n## {name}\nprojection: `{sql}`\n\
         ichiran_rows={ichiran_rows} new_db_rows={new_db_rows}\n\
         matched={matched} missing={missing} extra={extra} \
         elapsed={elapsed:.1}s",
    )
    .ok();
    Ok(r)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ichiran_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&args.ichiran_db)
        .await?;
    let new_db_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&args.new_db)
        .await?;

    let only: std::collections::HashSet<String> = args
        .only
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut report = BufWriter::new(File::create(&args.report)?);
    writeln!(
        report,
        "# multiset_compare — {} vs {}\n\n\
         Every row = u64 hash of all non-`id`-PK columns (each cast to \
         text in SQL). Multiset diff: matched = Σ min(ref[h], test[h]); \
         missing / extra = side-excess. Hash is 64-bit; collision \
         probability across ~10⁷ rows is ~10⁻¹¹.\n",
        args.ichiran_db, args.new_db,
    )?;

    let mut all = Vec::new();
    let t_total = Instant::now();
    for (name, sql) in TABLE_SELECTS {
        if !only.is_empty() && !only.contains(*name) {
            continue;
        }
        // ncols = comma count in the projection list. Brittle but
        // simple — the SELECT is hand-authored above.
        let head = sql.split(" FROM ").next().unwrap();
        let ncols = head.matches(',').count() + 1;
        println!("comparing {name}...");
        let r = compare_one(name, sql, ncols, &ichiran_pool, &new_db_pool, &mut report).await?;
        println!(
            "  {}: matched={} missing={} extra={} ({:.1}s)",
            r.name, r.matched, r.missing, r.extra, r.elapsed_s
        );
        all.push(r);
    }

    // Subset/superset call-out: which tables have rows that ichiran has
    // but kaniran doesn't (the bad case — kaniran is a strict subset for
    // those rows). Superset is fine (kaniran extra rows).
    writeln!(report, "\n# subset/superset summary\n")?;
    writeln!(
        report,
        "Kaniran is MISSING rows (subset of ichiran) iff `missing > 0`. \
         Extra rows (`extra > 0`) are fine.\n"
    )?;
    let missing_tables: Vec<&TableResult> = all.iter().filter(|r| r.missing > 0).collect();
    let clean_tables: Vec<&TableResult> = all.iter().filter(|r| r.missing == 0).collect();
    if missing_tables.is_empty() {
        writeln!(
            report,
            "No table is missing rows. Kaniran is a superset of ichiran across all {} tables.\n",
            all.len()
        )?;
    } else {
        writeln!(
            report,
            "Tables where kaniran is MISSING rows from ichiran:\n"
        )?;
        writeln!(report, "| table | missing | ichiran_rows | missing % |")?;
        writeln!(report, "|---|---:|---:|---:|")?;
        for r in &missing_tables {
            let pct = if r.ichiran_rows > 0 {
                100.0 * r.missing as f64 / r.ichiran_rows as f64
            } else {
                0.0
            };
            writeln!(
                report,
                "| {} | {} | {} | {:.4} |",
                r.name, r.missing, r.ichiran_rows, pct
            )?;
        }
        writeln!(
            report,
            "\nTables where kaniran is complete (no missing rows): {}\n",
            clean_tables
                .iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
                .join(", ")
        )?;
    }

    // Trailer overview.
    writeln!(report, "\n# overview\n")?;
    writeln!(
        report,
        "| table | status | ichiran_rows | new_db_rows | matched | missing | extra | elapsed |"
    )?;
    writeln!(report, "|---|---|---:|---:|---:|---:|---:|---:|")?;
    for r in &all {
        let status = if r.missing == 0 {
            if r.extra == 0 { "exact" } else { "superset" }
        } else {
            "MISSING"
        };
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} | {:.1}s |",
            r.name, status, r.ichiran_rows, r.new_db_rows, r.matched, r.missing, r.extra, r.elapsed_s,
        )?;
    }
    let total_m: i64 = all.iter().map(|r| r.matched).sum();
    let total_ro: i64 = all.iter().map(|r| r.missing).sum();
    let total_to: i64 = all.iter().map(|r| r.extra).sum();
    writeln!(
        report,
        "\n# overall\nmatched={total_m} missing={total_ro} extra={total_to} \
         elapsed={:.1}s",
        t_total.elapsed().as_secs_f64()
    )?;
    println!(
        "\nreport: {}\noverall matched={} missing={} extra={} elapsed={:.1}s",
        args.report.display(),
        total_m,
        total_ro,
        total_to,
        t_total.elapsed().as_secs_f64()
    );
    Ok(())
}
