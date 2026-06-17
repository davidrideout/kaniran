//! Output serializers, one file per format, behind a single [`render`].
//!
//! The segmentation pipeline runs once (via [`segment`]); the chosen
//! [`Format`] selects how its result is turned into the string we print. Each
//! format file owns its rendering and the helpers only it uses; the shared
//! machinery — the dispatch and the one pipeline run — lives here.
//!
//! The format-version split lives in the serializers: [`json_v1`] mirrors
//! ichiran's nested output, [`json_v2`] builds the flat view-model with real
//! JMdict sequence numbers.

mod json_v1;
mod json_v2;
mod romanize;

use std::error::Error;

use crate::conn::kani_context::KaniranContext;
use crate::conn::KaniDbError;
use crate::core::kani_romanize_method::KaniRomanizeMethod;
use crate::core::romanize::{romanize_star_, RomanizeStarSegment};

use json_v2::Detail;

/// One output flavor, as selected by `--format` (or the legacy `-i` / `-f`).
///
/// The `clap::ValueEnum` impl is gated behind the optional `clap` feature so
/// the library doesn't pull in a CLI parser by default; binaries that parse a
/// `--format` flag enable it.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Format {
    /// Plain romanization (the default).
    Romanize,
    /// Romanization plus a dictionary-info trailer.
    RomanizeInfo,
    /// ichiran-compatible nested JSON (legacy `-f`).
    V1,
    /// Flat v2 JSON: every field, real JMdict sequence numbers.
    V2,
    /// Flat v2 JSON, segmentation skeleton only — no glosses.
    V2Minimal,
}

/// Render `input` in the chosen `format`, returning the string to print.
///
/// `limit` is the segmentation beam width; it only affects the JSON formats.
///
/// # Errors
///
/// Propagates backend errors from segmentation and JSON serialization errors.
pub fn render(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    format: Format,
    limit: usize,
) -> Result<String, Box<dyn Error>> {
    match format {
        Format::Romanize => romanize::render(ctx, input, method),
        Format::RomanizeInfo => romanize::render_with_info(ctx, input, method),
        Format::V1 => json_v1::render(ctx, input, method, limit),
        Format::V2 => json_v2::render(ctx, input, method, limit, Detail::Full),
        Format::V2Minimal => json_v2::render(ctx, input, method, limit, Detail::Minimal),
    }
}

/// Run the segmentation pipeline once. Shared by the JSON serializers, which
/// both reshape the same `romanize*` result.
fn segment(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    limit: usize,
) -> Result<Vec<RomanizeStarSegment<()>>, KaniDbError> {
    romanize_star_(ctx, input, method, Some(limit), |_, _| ())
}
