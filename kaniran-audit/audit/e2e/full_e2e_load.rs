//! End-to-end runner for the complete kaniran data-loader chain
//! (JMdict + kanjidic + kanji stats), used by
//! `bookkeeping/e2e/full_loader_e2e.py` to diff every populated table
//! against the canonical `ichiran_260118` reference DB.
//!
//! Order matches `ichiran/maintenance:full-init` (`dict-load.lisp` /
//! `kanji.lisp`):
//!   1. [`load_jmdict`] — entry / kanji_text / kana_text / sense /
//!      gloss / sense_prop / conjugation / conj_prop /
//!      conj_source_reading / restricted_readings, then `load_extras`
//!      (errata + secondary conjugations).
//!   2. [`load_kanjidic`] — kanji / reading / okurigana / meaning.
//!   3. [`load_kanji_stats`] — `kanji.stat_common` /
//!      `kanji.stat_irregular` / `reading.stat_common`. Requires the
//!      JMdict tables populated, hence the strict order.
//!
//! Run:
//!   cargo run --release -p kaniran-audit --bin full_e2e_load -- \
//!       --db-url postgres:///ichiran_full_e2e \
//!       --jmdict-path bookkeeping/e2e/fixtures/JMdict_e_2026-02-22.xml \
//!       --kanjidic-path bookkeeping/e2e/fixtures/kanjidic2_2015-03-17.xml

use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::custom::load_custom_data::load_custom_data;
use kaniran_core::dict::load_best_readings::load_best_readings;
use kaniran_core::dict::load_jmdict::load_jmdict;
use kaniran_core::dict::recalc_entry_stats_all::recalc_entry_stats_all;
use kaniran_core::kanji::load_kanji_stats::load_kanji_stats;
use kaniran_core::kanji::load_kanjidic::load_kanjidic;
use kaniran_core::maintenance::add_errata::add_errata;

#[derive(Parser)]
struct Args {
    /// Postgres URL of the target DB.
    #[arg(long)]
    db_url: String,

    /// Path to the JMdict_e XML dump.
    #[arg(long)]
    jmdict_path: PathBuf,

    /// Path to a kanjidic2.xml dump.
    #[arg(long)]
    kanjidic_path: PathBuf,

    /// Skip load_extras (errata + secondary conjugations).
    #[arg(long)]
    skip_extras: bool,

    /// Skip load_kanjidic + load_kanji_stats.
    #[arg(long)]
    skip_kanji: bool,

    /// Resume mid-pipeline: skip load_jmdict entirely (it would
    /// TRUNCATE the tables), assume `load_conjugations` and
    /// `load_secondary_conjugations` already ran, and pick up at
    /// `load_custom_data` → `add_errata` → `recalc_entry_stats_all` →
    /// `ANALYZE`, then proceed with kanjidic. Use after a crash inside
    /// `load_extras` once the missing input file is fixed.
    #[arg(long)]
    resume_from_custom_data: bool,

    /// Run only `load_best_readings` against the existing DB and exit.
    /// Use after the rest of the chain already ran but missed this step
    /// (kaniran wired it in late; pre-fix runs left every
    /// `kanji_text.best_kana` / `kana_text.best_kanji` as NULL).
    #[arg(long)]
    just_best_readings: bool,

    /// Run only `load_kanji_stats` against the existing DB and exit.
    /// Use after `--just-best-readings` back-filled `kanji_text.best_kana`
    /// on a DB whose original load ran the chain in the wrong order —
    /// `load_kanji_stats` reads `best_kana` via `get_kanji_words`, so a
    /// pre-fix run got total=0 on every kanji and wrote `stat_common=0`
    /// everywhere.
    #[arg(long)]
    just_kanji_stats: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // pool_only_from_url skips the JMdict-backed cache populators
    // (build_no_conj_data / build_counter_cache / build_suffix_caches);
    // those query tables that are empty on a fresh DB.
    let ctx = KaniranContext::pool_only_from_url(&args.db_url).await?;

    let t0 = Instant::now();
    if args.just_best_readings {
        println!("e2e: load_best_readings (reset=true)");
        load_best_readings(&ctx, true).await?;
        println!("e2e: load_best_readings done in {:.1}s", t0.elapsed().as_secs_f64());
        return Ok(());
    }
    if args.just_kanji_stats {
        println!("e2e: load_kanji_stats");
        load_kanji_stats(&ctx).await?;
        println!("e2e: load_kanji_stats done in {:.1}s", t0.elapsed().as_secs_f64());
        return Ok(());
    }
    if args.resume_from_custom_data {
        // Pick up where load_extras crashed mid-stream. Skips
        // load_jmdict (would TRUNCATE everything), load_conjugations,
        // and load_secondary_conjugations (already in the DB).
        println!("e2e: resume — load_custom_data");
        load_custom_data(&ctx, &[], true).await?;
        println!("e2e: resume — add_errata");
        add_errata(&ctx).await?;
        println!("e2e: resume — recalc_entry_stats_all");
        recalc_entry_stats_all(&ctx).await?;
        println!("e2e: resume — load_best_readings");
        load_best_readings(&ctx, true).await?;
        println!("e2e: resume — ANALYZE");
        sqlx::query("ANALYZE").execute(&ctx.pool).await?;
        println!("e2e: resume done in {:.1}s", t0.elapsed().as_secs_f64());
    } else {
        println!("e2e: load_jmdict from {}", args.jmdict_path.display());
        load_jmdict(&ctx, &args.jmdict_path, !args.skip_extras).await?;
        println!("e2e: load_jmdict done in {:.1}s", t0.elapsed().as_secs_f64());
        // ichiran.lisp:34 — (load-best-readings :reset t) runs between
        // load-jmdict and load-kanjidic. Skip when --skip-extras since
        // it depends on the populated conjugation tables.
        if !args.skip_extras {
            let t = Instant::now();
            println!("e2e: load_best_readings");
            load_best_readings(&ctx, true).await?;
            println!("e2e: load_best_readings done in {:.1}s", t.elapsed().as_secs_f64());
        }
    }

    if !args.skip_kanji {
        let t1 = Instant::now();
        println!("e2e: load_kanjidic from {}", args.kanjidic_path.display());
        load_kanjidic(&ctx, &args.kanjidic_path).await?;
        println!("e2e: load_kanjidic done in {:.1}s", t1.elapsed().as_secs_f64());

        let t2 = Instant::now();
        println!("e2e: load_kanji_stats");
        load_kanji_stats(&ctx).await?;
        println!("e2e: load_kanji_stats done in {:.1}s", t2.elapsed().as_secs_f64());
    }

    println!("e2e: total {:.1}s", t0.elapsed().as_secs_f64());
    Ok(())
}
