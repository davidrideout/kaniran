//! Page handlers.
//!
//! Both routes render the same full reader page: `GET /?q=…` for shareable
//! links, `POST /` (form field `q`) for long pastes. The client script
//! re-submits the form over fetch and swaps the `#output` / `#stats`
//! regions out of the returned page — same handler, same template.
//!
//! Segmentation is synchronous, CPU-bound work, so it runs on the blocking
//! thread pool rather than on an async worker.

use axum::extract::{Form, Query, State};
use axum::response::Html;
use serde::Deserialize;

use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::methods::{hepburn_traditional, RomanizationMethod};
use kaniran_core::serializers::{v2_document, V2Options};

use crate::error::DemoError;
use crate::view::{reader_view, ReaderView};
use crate::AppState;

/// Seeded into the reader when the page is opened without a query, so the
/// page is immediately alive.
const SAMPLE_TEXT: &str = "私は日本語を勉強しています。暖かい春がやって来ました。";

/// `q` — the text to analyze; query string on GET, form field on POST.
#[derive(Debug, Deserialize)]
pub struct ReaderParams {
    #[serde(rename = "q")]
    query: Option<String>,
}

/// `GET /` — the reader page; `?q=` analyzes, otherwise the sample text.
pub async fn reader_get(
    State(state): State<AppState>,
    Query(params): Query<ReaderParams>,
) -> Result<Html<String>, DemoError> {
    let text = params
        .query
        .filter(|query| !query.trim().is_empty())
        .unwrap_or_else(|| SAMPLE_TEXT.to_owned());
    render_reader(state, text).await
}

/// `POST /` — form submit; empty input renders an empty analysis.
pub async fn reader_post(
    State(state): State<AppState>,
    Form(params): Form<ReaderParams>,
) -> Result<Html<String>, DemoError> {
    render_reader(state, params.query.unwrap_or_default()).await
}

/// Analyze off the async runtime, then render the full page.
async fn render_reader(state: AppState, text: String) -> Result<Html<String>, DemoError> {
    let templates = state.templates.clone();
    let view = tokio::task::spawn_blocking(move || build_view(&state, &text))
        .await
        .map_err(|join_error| DemoError::Internal(join_error.to_string()))??;
    let page = templates.render("reader.html", minijinja::context! { view })?;
    Ok(Html(page))
}

/// Segment `text` (best path of a beam-width-`limit` search) and shape the
/// page's view-model.
fn build_view(state: &AppState, text: &str) -> Result<ReaderView, DemoError> {
    let method = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
        hepburn_traditional(),
    ));
    let options = V2Options {
        include_paths: false,
        include_entries: true,
        include_furigana: true,
    };
    let started = std::time::Instant::now();
    let document = v2_document(&state.ctx, text, method, state.limit, options)?;
    let parse_time = started.elapsed();
    Ok(reader_view(&document, &state.levels, parse_time))
}
