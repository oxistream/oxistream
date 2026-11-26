use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

mod youtube;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(instance))
        .merge(youtube::router())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstanceInfo {
    /// Software name.
    pub name: &'static str,
    /// Server version.
    pub version: &'static str,
}

/// Basic information about this instance.
#[utoipa::path(
    get,
    path = "/instance",
    tag = "instance",
    responses((status = OK, description = "Instance information", body = InstanceInfo))
)]
async fn instance() -> Json<InstanceInfo> {
    Json(InstanceInfo {
        name: "Oxistream",
        version: env!("CARGO_PKG_VERSION"),
    })
}
