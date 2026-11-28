use std::time::Duration;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use oxistream_db::{AdminUpdate, DeleteOutcome, LockKey, Role, UserListQuery, Uuid};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::UserRole;
use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, ErrorCode};
use crate::auth::AdminUser;

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 100;
const MAX_SEARCH_LENGTH: usize = 100;
const DB_TIMEOUT: Duration = Duration::from_secs(2);
/// Set at build time, e.g. by the container image build.
const GIT_SHA: Option<&str> = option_env!("OXISTREAM_GIT_SHA");

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_users))
        .routes(routes!(update_user, delete_user))
        .routes(routes!(status))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// Username and password.
    Local,
    /// Single sign-on via `OpenID` Connect.
    Oidc,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserEntry {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub disabled: bool,
    pub auth_method: AuthMethod,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Most recent activity of any session; `null` without sessions.
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_seen_at: Option<OffsetDateTime>,
    pub subscriptions: i64,
}

impl From<oxistream_db::UserOverview> for AdminUserEntry {
    fn from(o: oxistream_db::UserOverview) -> Self {
        Self {
            id: o.user.id,
            username: o.user.username,
            role: o.user.role.into(),
            disabled: o.user.disabled,
            auth_method: if o.sso {
                AuthMethod::Oidc
            } else {
                AuthMethod::Local
            },
            created_at: o.user.created_at,
            last_seen_at: o.last_seen_at,
            subscriptions: o.subscriptions,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserPage {
    pub users: Vec<AdminUserEntry>,
    /// Users matching the search.
    pub total: i64,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ListParams {
    /// Only usernames containing this, ignoring case. At most 100 characters.
    q: Option<String>,
    /// Page size, 1 to 100; default 50.
    limit: Option<u32>,
    /// Users to skip; default 0.
    offset: Option<u32>,
}

/// Users, newest first. Admins only.
#[utoipa::path(
    get,
    path = "/admin/users",
    tag = "admin",
    security(("session" = [])),
    params(ListParams),
    responses(
        (status = OK, body = AdminUserPage),
        (status = BAD_REQUEST, description = "Invalid limit or search", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Not an admin (`forbidden`)", body = ErrorBody),
    )
)]
async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<ListParams>,
) -> Result<Json<AdminUserPage>, ApiError> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(ApiError::bad_request(format!(
            "`limit` must be between 1 and {MAX_LIMIT}"
        )));
    }
    let search = params
        .q
        .map(|q| q.trim().to_owned())
        .filter(|q| !q.is_empty());
    if search
        .as_ref()
        .is_some_and(|q| q.chars().count() > MAX_SEARCH_LENGTH)
    {
        return Err(ApiError::bad_request(format!(
            "`q` must be at most {MAX_SEARCH_LENGTH} characters"
        )));
    }
    let page = state
        .auth
        .users()
        .list(&UserListQuery {
            search,
            limit,
            offset: params.offset.unwrap_or(0),
        })
        .await?;
    Ok(Json(AdminUserPage {
        users: page.users.into_iter().map(Into::into).collect(),
        total: page.total,
    }))
}

/// Fields to change; absent fields stay as they are.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUserUpdate {
    pub role: Option<AdminRole>,
    /// Disabling ends all of the user's sessions.
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdminRole {
    User,
    Admin,
}

impl From<AdminRole> for Role {
    fn from(role: AdminRole) -> Self {
        match role {
            AdminRole::User => Self::User,
            AdminRole::Admin => Self::Admin,
        }
    }
}

fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, "user not found")
}

fn own_account() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        ErrorCode::OwnAccount,
        "admins cannot change their own account here",
    )
}

fn last_admin() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        ErrorCode::LastAdmin,
        "the instance needs at least one active admin",
    )
}

fn parse_id(id: &str) -> Result<Uuid, ApiError> {
    Uuid::try_parse(id).map_err(|_| not_found())
}

fn applied(update: AdminUpdate) -> Result<oxistream_db::User, ApiError> {
    match update {
        AdminUpdate::Updated(user) => Ok(user),
        AdminUpdate::NotFound => Err(not_found()),
        AdminUpdate::LastAdmin => Err(last_admin()),
    }
}

/// Change a user's role or disable them. Admins cannot change their own account, and the last
/// active admin is never demoted or disabled. With an OIDC admin group, the role of single
/// sign-on users is synced again on their next sign-in.
#[utoipa::path(
    patch,
    path = "/admin/users/{id}",
    tag = "admin",
    security(("session" = [])),
    params(("id" = String, Path, description = "User ID")),
    request_body = AdminUserUpdate,
    responses(
        (status = NO_CONTENT, description = "Updated"),
        (status = BAD_REQUEST, description = "Malformed body", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Not an admin (`forbidden`), or foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "Unknown user", body = ErrorBody),
        (status = CONFLICT, description = "Own account (`own_account`) or last active admin (`last_admin`)", body = ErrorBody),
    )
)]
async fn update_user(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<String>,
    body: Result<Json<AdminUserUpdate>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&id)?;
    let Json(update) = body?;
    if id == admin.id {
        return Err(own_account());
    }
    let users = state.auth.users();
    if let Some(role) = update.role {
        let user = applied(users.admin_set_role(id, role.into()).await?)?;
        tracing::info!(user_id = %id, admin_id = %admin.id, role = %user.role, "role changed");
    }
    if let Some(disabled) = update.disabled {
        applied(users.admin_set_disabled(id, disabled).await?)?;
        tracing::info!(user_id = %id, admin_id = %admin.id, disabled, "user access changed");
    }
    if update.role.is_none() && update.disabled.is_none() && users.get(id).await?.is_none() {
        return Err(not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Delete a user with everything they own, ending their sessions. Admins delete their own
/// account via `DELETE /account`.
#[utoipa::path(
    delete,
    path = "/admin/users/{id}",
    tag = "admin",
    security(("session" = [])),
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = NO_CONTENT, description = "Deleted"),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Not an admin (`forbidden`), or foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "Unknown user", body = ErrorBody),
        (status = CONFLICT, description = "Own account (`own_account`) or last admin (`last_admin`)", body = ErrorBody),
    )
)]
async fn delete_user(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&id)?;
    if id == admin.id {
        return Err(own_account());
    }
    match state.auth.users().delete(id).await? {
        DeleteOutcome::Deleted => {}
        DeleteOutcome::NotFound => return Err(not_found()),
        DeleteOutcome::LastAdmin => return Err(last_admin()),
    }
    tracing::info!(user_id = %id, admin_id = %admin.id, "user deleted by admin");
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminStatus {
    pub version: &'static str,
    /// Commit the server was built from, if known.
    pub git_sha: Option<&'static str>,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    pub uptime_seconds: u64,
    pub database: DatabaseStatus,
    /// `null` while the database is unreachable.
    pub counts: Option<InstanceCounts>,
    pub feed: FeedWorkerStatus,
    /// Cached player responses of this replica.
    pub player_cache_entries: u64,
    /// `null` if `rustypipe-botguard` was not found.
    pub botguard_version: Option<String>,
    pub auth: AuthStatus,
    pub sponsorblock: bool,
    /// ISO country code of the trending pages.
    pub trending_region: String,
    pub rate_limit: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DatabaseStatus {
    pub reachable: bool,
    /// Postgres `server_version`.
    pub version: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstanceCounts {
    pub users: i64,
    /// Admins that are not disabled.
    pub admins: i64,
    pub disabled_users: i64,
    /// Channels with at least one subscriber.
    pub channels: i64,
    pub subscriptions: i64,
    pub playlists: i64,
    /// Cached video metadata.
    pub videos: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedWorkerStatus {
    /// Fetch interval; `null` if no worker runs on this replica.
    pub interval_minutes: Option<u64>,
    /// A replica holds the worker lock, i.e. a run is in progress.
    pub running: Option<bool>,
    /// Subscribed channels never fetched successfully.
    pub pending: Option<i64>,
    /// Subscribed channels whose last fetch failed.
    pub failing: Option<i64>,
    /// Subscribed channels due for the next run.
    pub due: Option<i64>,
    /// Most recent fetch attempt of any subscribed channel, by any replica.
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_checked_at: Option<OffsetDateTime>,
    /// Last run of this replica's worker.
    pub last_run: Option<FeedRun>,
    /// Last run of this replica's worker that failed with a database error.
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_error_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedRun {
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
    pub channels_fetched: usize,
    pub channels_failed: usize,
    pub playlists_fetched: usize,
    pub playlists_failed: usize,
    /// Stopped early because YouTube is blocking requests.
    pub rate_limited: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[allow(clippy::struct_excessive_bools)]
pub struct AuthStatus {
    pub local_enabled: bool,
    pub registration_enabled: bool,
    pub require_login: bool,
    pub oidc_enabled: bool,
    /// Provider discovery succeeded; `false` without single sign-on.
    pub oidc_discovered: bool,
}

async fn timed<T>(
    future: impl Future<Output = Result<T, oxistream_db::Error>>,
) -> Result<T, ApiError> {
    match tokio::time::timeout(DB_TIMEOUT, future).await {
        Ok(result) => Ok(result?),
        Err(_) => Err(ApiError::internal()),
    }
}

/// Instance status: build, database, feed worker and configuration. Caches and the feed
/// worker's last run are those of the replica that answers. Admins only.
#[utoipa::path(
    get,
    path = "/admin/status",
    tag = "admin",
    security(("session" = [])),
    responses(
        (status = OK, body = AdminStatus),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Not an admin (`forbidden`)", body = ErrorBody),
    )
)]
async fn status(State(state): State<AppState>, _admin: AdminUser) -> Json<AdminStatus> {
    let report = state.feed.report();
    let db_version = timed(state.stats.server_version()).await.ok();
    let reachable = db_version.is_some();
    let (counts, feed_status, running, due) = if reachable {
        let counts = timed(state.stats.counts()).await.ok();
        let feed_status = timed(state.feed.status()).await.ok();
        let running = timed(state.stats.lock_held(LockKey::FeedWorker)).await.ok();
        let due = match report.interval {
            Some(interval) => timed(state.feed.due_count(interval)).await.ok(),
            None => None,
        };
        (counts, feed_status, running, due)
    } else {
        (None, None, None, None)
    };
    let config = state.auth.config();
    let now = OffsetDateTime::now_utc();
    Json(AdminStatus {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: GIT_SHA.filter(|sha| !sha.is_empty()),
        started_at: state.started_at,
        uptime_seconds: u64::try_from((now - state.started_at).whole_seconds()).unwrap_or(0),
        database: DatabaseStatus {
            reachable,
            version: db_version,
        },
        counts: counts.map(|c| InstanceCounts {
            users: c.users,
            admins: c.admins,
            disabled_users: c.disabled_users,
            channels: feed_status.as_ref().map_or(0, |f| f.channels),
            subscriptions: c.subscriptions,
            playlists: c.playlists,
            videos: c.videos,
        }),
        feed: FeedWorkerStatus {
            interval_minutes: report.interval.map(|i| i.as_secs() / 60),
            running,
            pending: feed_status.as_ref().map(|f| f.pending),
            failing: feed_status.as_ref().map(|f| f.failing),
            due,
            last_checked_at: feed_status.and_then(|f| f.last_checked_at),
            last_run: report
                .last_run_at
                .zip(report.last_run)
                .map(|(finished_at, run)| FeedRun {
                    finished_at,
                    channels_fetched: run.fetched,
                    channels_failed: run.failed,
                    playlists_fetched: run.playlists_fetched,
                    playlists_failed: run.playlists_failed,
                    rate_limited: run.rate_limited,
                }),
            last_error_at: report.last_error_at,
        },
        player_cache_entries: state.youtube.player_cache_entries(),
        botguard_version: state.youtube.botguard_version().await,
        auth: AuthStatus {
            local_enabled: config.local_enabled,
            registration_enabled: config.local_enabled && config.registration_enabled,
            require_login: config.require_login,
            oidc_enabled: config.oidc.is_some(),
            oidc_discovered: state
                .auth
                .oidc()
                .is_some_and(crate::auth::oidc::Oidc::is_ready),
        },
        sponsorblock: state.sponsorblock.enabled(),
        trending_region: serde_json::to_value(state.youtube.trending_region())
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
        rate_limit: state.rate_limit.is_some(),
    })
}
