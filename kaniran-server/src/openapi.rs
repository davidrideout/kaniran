//! The OpenAPI document, assembled from the `#[utoipa::path]`-annotated
//! handlers and `ToSchema` types. Served as JSON at
//! `/api-docs/openapi.json` and rendered by Swagger UI at `/docs`.

use utoipa::OpenApi;

/// Root OpenAPI definition for the service. `version` defaults to the
/// crate version; the title/description come from here.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "kaniran-server",
        description = "HTTP API for kaniran Japanese segmentation and romanization."
    ),
    paths(
        crate::handlers::health,
        crate::handlers::segment_get,
        crate::handlers::segment_post,
    ),
    components(schemas(crate::handlers::SegmentParams, crate::error::ErrorBody)),
    tags(
        (name = "segment", description = "Segmentation and romanization"),
        (name = "ops", description = "Operational endpoints"),
    )
)]
pub struct ApiDoc;
