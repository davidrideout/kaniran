//! End-to-end runner for the kanjidic loader: drives
//! [`kaniran_core::kanji::load_kanjidic::load_kanjidic`] (and
//! optionally [`kaniran_core::kanji::load_kanji_stats::load_kanji_stats`])
//! against an arbitrary Postgres URL.
//!
//! Run:
//!   cargo run --release -p kaniran-audit --bin e2e_load_kanjidic -- \
//!       --db-url postgres://david@localhost/ichiran_kanji_e2e \
//!       --path /Users/david/Desktop/work/ichirust/fixtures/kanjidic2.xml \
//!       [--load-stats]

use std::path::PathBuf;

use clap::Parser;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::kanji::load_kanji_stats::load_kanji_stats;
use kaniran_core::kanji::load_kanjidic::load_kanjidic;

#[derive(Parser)]
struct Args {
    /// Postgres URL of the target DB (e.g. postgres://user@host/dbname).
    #[arg(long)]
    db_url: String,

    /// Path to a kanjidic2.xml dump.
    #[arg(long)]
    path: PathBuf,

    /// Also run load_kanji_stats after load_kanjidic. Requires the
    /// JMdict-derived tables (entry, kanji_text, kana_text) to be
    /// populated in the target DB.
    #[arg(long)]
    load_stats: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // Skip the JMdict-backed cache populators: load_kanjidic doesn't read
    // them, and load_kanji_stats reads JMdict tables directly via plain
    // queries (no cache dependency).
    let ctx = KaniranContext::pool_only_from_url(&args.db_url).await?;
    println!("e2e: load_kanjidic from {}", args.path.display());
    load_kanjidic(&ctx, &args.path).await?;
    if args.load_stats {
        println!("e2e: load_kanji_stats");
        load_kanji_stats(&ctx).await?;
    }
    Ok(())
}
