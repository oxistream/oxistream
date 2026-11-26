use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

mod auth;
mod cursor;
mod follows;
mod history;
mod likes;
mod manifest;
mod oidc;
mod playlists;
mod storyboard;
mod subscriptions;
mod youtube;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(instance))
        .merge(auth::router())
        .merge(oidc::router())
        .merge(youtube::router())
        .merge(manifest::router())
        .merge(storyboard::router())
        .merge(subscriptions::router())
        .merge(history::router())
        .merge(playlists::router())
        .merge(follows::router())
        .merge(likes::router())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstanceInfo {
    /// Software name.
    pub name: &'static str,
    /// Server version.
    pub version: &'static str,
    pub auth: AuthCapabilities,
}

/// Which sign-in methods this instance offers.
#[derive(Debug, Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct AuthCapabilities {
    /// Local accounts with username and password.
    pub local_enabled: bool,
    /// New local accounts can be created.
    pub registration_enabled: bool,
    /// Anonymous users cannot use the API or proxy.
    pub require_login: bool,
    /// Single sign-on via `OpenID` Connect, started at `/api/v0/auth/oidc/login`.
    pub oidc_enabled: bool,
    /// Label of the single sign-on button; `null` without single sign-on.
    pub oidc_name: Option<String>,
}

/// Basic information about this instance.
#[utoipa::path(
    get,
    path = "/instance",
    tag = "instance",
    responses((status = OK, description = "Instance information", body = InstanceInfo))
)]
async fn instance(State(state): State<AppState>) -> Json<InstanceInfo> {
    let config = state.auth.config();
    Json(InstanceInfo {
        name: "Oxistream",
        version: env!("CARGO_PKG_VERSION"),
        auth: AuthCapabilities {
            local_enabled: config.local_enabled,
            registration_enabled: config.local_enabled && config.registration_enabled,
            require_login: config.require_login,
            oidc_enabled: config.oidc.is_some(),
            oidc_name: config.oidc.as_ref().map(|oidc| oidc.name.clone()),
        },
    })
}
