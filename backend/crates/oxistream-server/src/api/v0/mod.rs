use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::retention::Retention;

mod account;
mod auth;
mod blocks;
mod cursor;
pub(crate) mod feed_token;
mod follows;
mod groups;
pub(crate) mod hidden;
mod history;
mod likes;
mod manifest;
mod oidc;
mod playlists;
mod queue;
pub(super) mod rooms;
mod settings;
mod shorts;
mod sponsorblock;
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
        .merge(groups::router())
        .merge(history::router())
        .merge(playlists::router())
        .merge(follows::router())
        .merge(blocks::router())
        .merge(hidden::router())
        .merge(likes::router())
        .merge(queue::router())
        .merge(sponsorblock::router())
        .merge(settings::router())
        .merge(feed_token::router())
        .merge(account::router())
        .merge(shorts::router())
        .merge(rooms::router())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstanceInfo {
    /// Software name.
    pub name: &'static str,
    /// Server version.
    pub version: &'static str,
    pub auth: AuthCapabilities,
    /// SponsorBlock segments are available.
    pub sponsorblock: bool,
    /// Longest watch history retention users may pick; guests' local history follows it, too.
    pub history_max_retention: Retention,
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
        sponsorblock: state.sponsorblock.enabled(),
        history_max_retention: state.history_max_retention,
    })
}
