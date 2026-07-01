//! `kaniran-cli` — Rust transliteration of `ichiran/cli` (`cli.lisp`).
//!
//! Parses CLI options and romanizes the free-argument input. The output
//! format (`--format`, or the legacy `-i` / `-f`) selects how the result is
//! rendered; the rendering itself lives in [`render`].

use std::io::Write;

use clap::Parser;

use kaniran_core::serializers::{render, Format};

// mimalloc over the system allocator: measured 1.57x end-to-end on the
// allocation-bound segmentation pipeline (perf pass 2026-06-10).
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use kaniran_core::characters::text::join;
use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::methods::hepburn_traditional;
use kaniran_core::core::methods::RomanizationMethod;

// cli.lisp:8-31 (opts:define-opts) — :eval (cli.lisp:13) omitted; :help
// (cli.lisp:9) is clap's built-in.
#[derive(Parser)]
#[command(
    name = "kaniran-cli",
    about = "Command line interface for Ichiran",
    long_about = "Command line interface for Ichiran.\n\n\
        By default romanizes the input; --format selects the output (see --format below).\n\n\
        Examples:\n  \
        kaniran-cli \"一覧は最高だぞ\"\n  \
        kaniran-cli --format v2 \"食べたい\"\n  \
        kaniran-cli --format v2-minimal -l 5 \"食べたくなかった\""
)]
struct Cli {
    /// full split info as JSON (alias for --format v1)
    #[arg(short = 'f', long = "full")]
    full: bool,
    /// keep the top N segmentations — the search beam width; affects the JSON formats [Example: kaniran-cli --format v2 -l 5 "一覧は最高だぞ"]
    #[arg(short = 'l', long = "limit", default_value_t = 1, value_name = "LIMIT")]
    limit: usize,
    /// select the output format (overrides -f); see the possible values below
    #[arg(long = "format", value_enum)]
    format: Option<Format>,
    /// input
    input: Vec<String>,
}

impl Cli {
    /// `--format` wins; otherwise the legacy `-f` flag maps onto it.
    fn resolved_format(&self) -> Format {
        self.format.unwrap_or(if self.full {
            Format::V1
        } else {
            Format::Romanize
        })
    }
}

// cli.lisp:48 (main)
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Cli::parse();
    // (load-connection-from-env)
    let ctx = KaniranContext::from_env()?;
    // method defaults to *default-romanization-method* (= *hepburn-traditional*).
    let method =
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
    // (join " " free-args)
    let input = join(" ", &options.input);
    // The pipeline runs once inside `render`; the format picks the rendering.
    // `include_paths = true` keeps the CLI's all-readings v2 output (the HTTP
    // API defaults it off).
    let output = render(
        &ctx,
        &input,
        method,
        options.resolved_format(),
        options.limit,
        true,
    )?;
    print!("{output}");
    // (terpri) (finish-output)
    println!();
    std::io::stdout().flush()?;
    Ok(())
}
