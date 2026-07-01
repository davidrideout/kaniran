//! Request handlers.
//!
//! Segmentation is synchronous, CPU-bound work, so it runs on the blocking
//! thread pool ([`tokio::task::spawn_blocking`]) rather than on an async
//! worker. The shared [`KaniranContext`] is an `Arc`, so each request clones
//! a refcount, not the caches.

use axum::extract::{Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::Json;

use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::methods::{hepburn_traditional, RomanizationMethod};
use kaniran_core::serializers::{render, Format};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::error::{ApiError, ErrorBody};
use crate::AppState;

/// Query params (`GET`) / JSON body (`POST`) for the segment endpoint.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SegmentParams {
    /// The Japanese text to segment/romanize.
    #[schema(example = "一覧は最高だぞ")]
    #[param(example = "一覧は最高だぞ")]
    pub text: String,
    /// Output format. One of `romanize`, `romanize-info`, `v1`, `v2`,
    /// `v2-minimal`; defaults to `v2` when omitted.
    #[serde(default)]
    #[schema(example = "v2")]
    #[param(example = "v2")]
    pub format: Option<String>,
    /// Segmentation beam width; falls back to the server's configured default.
    #[serde(default)]
    #[schema(example = 5)]
    #[param(example = 5)]
    pub limit: Option<usize>,
    /// Include the `paths` array — every kept segmentation reading — in `v2` /
    /// `v2-minimal` output. Defaults to `false`; has no effect on other formats.
    #[serde(default)]
    #[schema(example = false)]
    #[param(example = false)]
    pub include_paths: Option<bool>,
}

/// Liveness probe.
#[utoipa::path(
    get,
    path = "/health",
    tag = "ops",
    responses((status = 200, description = "Service is up", body = String))
)]
pub async fn health() -> &'static str {
    "ok"
}

/// `GET /segment?text=…&format=…&limit=…`.
#[utoipa::path(
    get,
    path = "/segment",
    tag = "segment",
    params(SegmentParams),
    responses(
        (status = 200, description = "Rendered segmentation: JSON for v1/v2/v2-minimal, plain text for romanize/romanize-info"),
        (status = 400, description = "Empty text or unknown format", body = ErrorBody),
    )
)]
pub async fn segment_get(
    State(state): State<AppState>,
    Query(params): Query<SegmentParams>,
) -> Result<Response, ApiError> {
    run_segment(state, params).await
}

/// `POST /segment` with a JSON `SegmentParams` body.
#[utoipa::path(
    post,
    path = "/segment",
    tag = "segment",
    request_body = SegmentParams,
    responses(
        (status = 200, description = "Rendered segmentation: JSON for v1/v2/v2-minimal, plain text for romanize/romanize-info"),
        (status = 400, description = "Empty text or unknown format", body = ErrorBody),
        (status = 422, description = "Malformed JSON body"),
    )
)]
pub async fn segment_post(
    State(state): State<AppState>,
    Json(params): Json<SegmentParams>,
) -> Result<Response, ApiError> {
    run_segment(state, params).await
}

/// Shared body for both segment routes: validate, run the pipeline off the
/// async runtime, and tag the response content type by format.
async fn run_segment(state: AppState, params: SegmentParams) -> Result<Response, ApiError> {
    if params.text.trim().is_empty() {
        return Err(ApiError::BadRequest("`text` must not be empty".to_owned()));
    }
    let format = parse_format(params.format.as_deref())?;
    let limit = params.limit.unwrap_or(state.default_limit);
    let include_paths = params.include_paths.unwrap_or(false);
    let ctx = state.ctx.clone();
    let text = params.text;

    // render() is sync and CPU-bound; keep it off the async workers. Its
    // error is `Box<dyn Error>` (not `Send`), so flatten it to a String
    // inside the closure before it crosses the task boundary.
    let rendered = tokio::task::spawn_blocking(move || {
        let method = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
            hepburn_traditional(),
        ));
        render(&ctx, &text, method, format, limit, include_paths).map_err(|err| err.to_string())
    })
    .await
    .map_err(|join_err| ApiError::Internal(join_err.to_string()))?
    .map_err(ApiError::Render)?;

    let content_type = if is_json(format) {
        "application/json"
    } else {
        "text/plain; charset=utf-8"
    };
    Ok(([(CONTENT_TYPE, content_type)], rendered).into_response())
}

/// Map a format string to [`Format`]; `None`/empty defaults to `v2`.
fn parse_format(raw: Option<&str>) -> Result<Format, ApiError> {
    let Some(name) = raw.map(str::trim).filter(|name| !name.is_empty()) else {
        return Ok(Format::V2);
    };
    match name.to_ascii_lowercase().as_str() {
        "romanize" | "romaji" => Ok(Format::Romanize),
        "romanize-info" | "info" => Ok(Format::RomanizeInfo),
        "v1" | "full" => Ok(Format::V1),
        "v2" => Ok(Format::V2),
        "v2-minimal" | "minimal" => Ok(Format::V2Minimal),
        other => Err(ApiError::BadRequest(format!(
            "unknown format `{other}` (expected: romanize, romanize-info, v1, v2, v2-minimal)"
        ))),
    }
}

/// Whether a format renders JSON (vs. plain text), used to set the
/// response content type.
fn is_json(format: Format) -> bool {
    matches!(format, Format::V1 | Format::V2 | Format::V2Minimal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_format_defaults_to_v2() {
        assert!(matches!(parse_format(None), Ok(Format::V2)));
        assert!(matches!(parse_format(Some("  ")), Ok(Format::V2)));
    }

    #[test]
    fn parse_format_is_case_and_alias_tolerant() {
        assert!(matches!(parse_format(Some("V1")), Ok(Format::V1)));
        assert!(matches!(parse_format(Some("full")), Ok(Format::V1)));
        assert!(matches!(parse_format(Some("Minimal")), Ok(Format::V2Minimal)));
        assert!(matches!(parse_format(Some("romaji")), Ok(Format::Romanize)));
    }

    #[test]
    fn parse_format_rejects_unknown() {
        assert!(matches!(parse_format(Some("v3")), Err(ApiError::BadRequest(_))));
    }

    #[test]
    fn json_formats_are_tagged() {
        assert!(is_json(Format::V2));
        assert!(!is_json(Format::Romanize));
    }
}
