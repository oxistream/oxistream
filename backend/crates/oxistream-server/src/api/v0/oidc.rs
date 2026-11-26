use axum::Json;
use axum::extract::rejection::FormRejection;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use oxistream_db::{BackchannelLogout, LogoutOutcome, Role, User};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::auth::ClientAddr;
use crate::auth::oidc::backchannel::Logout;
use crate::auth::oidc::{
    CallbackParams, LoginError, VerifiedLogin, claims, pending, safe_redirect,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(oidc_login))
        .routes(routes!(oidc_callback))
        .routes(routes!(backchannel_logout))
}

#[derive(Debug, Deserialize, ToSchema)]
struct LogoutTokenForm {
    /// Signed logout token (JWT) from the provider.
    logout_token: String,
}

/// OAuth 2.0 error response, as the back-channel logout spec asks for.
#[derive(Debug, Serialize, ToSchema)]
struct OAuthError {
    error: &'static str,
    error_description: String,
}

fn no_store(status: StatusCode, body: Option<OAuthError>) -> Response {
    let headers = [(header::CACHE_CONTROL, "no-store")];
    match body {
        Some(body) => (status, headers, Json(body)).into_response(),
        None => (status, headers).into_response(),
    }
}

fn invalid_request(description: impl Into<String>) -> Response {
    no_store(
        StatusCode::BAD_REQUEST,
        Some(OAuthError {
            error: "invalid_request",
            error_description: description.into(),
        }),
    )
}

/// Back-channel logout: the provider ends sessions server to server.
///
/// Signs out the sessions with the token's `sid`, or all sessions of its `sub`. Configure
/// `{PUBLIC_URL}/api/v0/auth/oidc/backchannel-logout` as the client's back-channel logout URL.
/// Needs no `Origin` header or session.
#[utoipa::path(
    post,
    path = "/auth/oidc/backchannel-logout",
    tag = "auth",
    request_body(content = LogoutTokenForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = OK, description = "Matching sessions ended"),
        (status = BAD_REQUEST, description = "Invalid, replayed or unexpected logout token", body = OAuthError),
        (status = SERVICE_UNAVAILABLE, description = "Provider not discovered yet", body = OAuthError),
    )
)]
async fn backchannel_logout(
    State(state): State<AppState>,
    form: Result<Form<LogoutTokenForm>, FormRejection>,
) -> Response {
    let Some(oidc) = state.auth.oidc() else {
        return invalid_request("single sign-on is not configured");
    };
    let Ok(Form(LogoutTokenForm { logout_token })) = form else {
        return invalid_request("expected a form-encoded logout_token");
    };
    let logout = match oidc
        .validate_logout_token(&logout_token, pending::now())
        .await
    {
        Ok(logout) => logout,
        Err(Some(err)) => {
            tracing::warn!(reason = %err, "rejected back-channel logout token");
            return invalid_request(err.to_string());
        }
        Err(None) => {
            return no_store(
                StatusCode::SERVICE_UNAVAILABLE,
                Some(OAuthError {
                    error: "temporarily_unavailable",
                    error_description: "provider not discovered yet".into(),
                }),
            );
        }
    };
    match end_sessions(&state, &oidc.config().issuer, &logout).await {
        Ok(Some(count)) => {
            tracing::info!(count, "sessions ended by back-channel logout");
            no_store(StatusCode::OK, None)
        }
        Ok(None) => invalid_request("logout token was already used"),
        Err(err) => {
            tracing::error!(error = ?err, "database error during back-channel logout");
            no_store(StatusCode::INTERNAL_SERVER_ERROR, None)
        }
    }
}

/// `None` if the token was replayed.
async fn end_sessions(
    state: &AppState,
    issuer: &str,
    logout: &Logout,
) -> Result<Option<u64>, oxistream_db::Error> {
    let expires_at = OffsetDateTime::from_unix_timestamp(
        i64::try_from(logout.replayable_until).unwrap_or(i64::MAX),
    )
    .unwrap_or(OffsetDateTime::now_utc() + time::Duration::hours(1));
    let outcome = state
        .auth
        .sessions()
        .end_sessions_for_logout(&BackchannelLogout {
            jti: &logout.jti,
            expires_at,
            issuer,
            sid: logout.sid.as_deref(),
            subject: logout.sub.as_deref(),
        })
        .await?;
    Ok(match outcome {
        LogoutOutcome::Replayed => None,
        LogoutOutcome::Ended(count) => Some(count),
    })
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct LoginParams {
    /// Same-origin path to continue to after signing in.
    redirect: Option<String>,
}

fn redirect(location: &str, cookies: impl IntoIterator<Item = HeaderValue>) -> Response {
    let mut response = (
        StatusCode::FOUND,
        [
            (header::LOCATION, location),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response();
    for cookie in cookies {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    response
}

/// `/login?error=…`, keeping the target so the user can try again.
fn error_location(error: LoginError, target: &str) -> String {
    let mut query = openidconnect::url::form_urlencoded::Serializer::new(String::new());
    query.append_pair("error", error.code());
    if target != "/" {
        query.append_pair("redirect", target);
    }
    format!("/login?{}", query.finish())
}

/// Start single sign-on with the configured provider.
///
/// Redirects the browser to the provider. Failures redirect to `/login?error=` with
/// `oidc_disabled` or `oidc_unavailable`.
#[utoipa::path(
    get,
    path = "/auth/oidc/login",
    tag = "auth",
    params(LoginParams),
    responses(
        (status = FOUND, description = "To the provider's authorization endpoint, or to `/login?error=`",
            headers(
                ("location" = String),
                ("set-cookie" = String, description = "Short-lived `oxistream_oidc` cookie holding the login state"),
            )),
    )
)]
async fn oidc_login(State(state): State<AppState>, Query(params): Query<LoginParams>) -> Response {
    let target = safe_redirect(params.redirect.as_deref());
    let Some(oidc) = state.auth.oidc() else {
        return redirect(&error_location(LoginError::Disabled, &target), []);
    };
    match oidc.start_login(target.clone()) {
        Ok((url, value)) => redirect(
            url.as_str(),
            [pending::cookie(&value, state.auth.config().cookie_secure)],
        ),
        Err(err) => redirect(&error_location(err, &target), []),
    }
}

/// Where the provider sends the browser back after sign-in.
///
/// On success, starts a session and redirects to the target given to `/auth/oidc/login`.
/// Unknown identities get a new account; the first account of an instance becomes an admin.
/// Failures redirect to `/login?error=` with `oidc_denied`, `oidc_failed`, `oidc_unavailable`,
/// `oidc_disabled` or `account_disabled`.
#[utoipa::path(
    get,
    path = "/auth/oidc/callback",
    tag = "auth",
    params(CallbackParams),
    responses(
        (status = FOUND, description = "To the sign-in target, or to `/login?error=`",
            headers(
                ("location" = String),
                ("set-cookie" = String, description = "`oxistream_session` cookie; clears `oxistream_oidc`"),
            )),
    )
)]
async fn oidc_callback(
    State(state): State<AppState>,
    ClientAddr(ip): ClientAddr,
    headers: HeaderMap,
    Query(params): Query<CallbackParams>,
) -> Response {
    let auth = &state.auth;
    let clear = pending::clear_cookie(auth.config().cookie_secure);
    let Some(oidc) = auth.oidc() else {
        return redirect(&error_location(LoginError::Disabled, "/"), [clear]);
    };
    let Some(pending) = oidc.pending_login(pending::from_headers(&headers)) else {
        tracing::info!("OIDC callback without a valid login state");
        return redirect(&error_location(LoginError::Failed, "/"), [clear]);
    };
    let target = pending.redirect.clone();
    let result = async {
        let login = oidc.finish_login(params, pending).await?;
        sign_in(&state, login, &headers, ip).await
    }
    .await;
    match result {
        Ok(session) => redirect(&target, [session, clear]),
        Err(err) => redirect(&error_location(err, &target), [clear]),
    }
}

async fn sign_in(
    state: &AppState,
    login: VerifiedLogin,
    headers: &HeaderMap,
    ip: Option<std::net::IpAddr>,
) -> Result<HeaderValue, LoginError> {
    let auth = &state.auth;
    let db_failed = |err: oxistream_db::Error| {
        tracing::error!(error = ?err, "database error during OIDC sign-in");
        LoginError::Failed
    };
    let existing = auth
        .users()
        .find_by_identity(&login.identity)
        .await
        .map_err(db_failed)?;
    let user = if let Some(user) = existing {
        user
    } else {
        let base = claims::username_base(&login.claims);
        let user = auth
            .users()
            .provision(
                &login.identity,
                &|attempt| claims::username_candidate(&base, attempt),
                login.admin.unwrap_or(false),
            )
            .await
            .map_err(db_failed)?;
        tracing::info!(user_id = %user.id, role = %user.role, "account provisioned via OIDC");
        user
    };
    if user.disabled {
        return Err(LoginError::AccountDisabled);
    }
    let user = sync_role(state, user, login.admin)
        .await
        .map_err(db_failed)?;
    auth.start_oidc_session(&user, headers, ip, login.sid, Some(login.id_token))
        .await
        .map_err(|_| LoginError::Failed)
}

/// Applies the admin group membership; the last admin keeps the role.
async fn sync_role(
    state: &AppState,
    user: User,
    admin: Option<bool>,
) -> Result<User, oxistream_db::Error> {
    let Some(admin) = admin else {
        return Ok(user);
    };
    let role = if admin { Role::Admin } else { Role::User };
    if user.role == role {
        return Ok(user);
    }
    let updated = state
        .auth
        .users()
        .sync_role(user.id, role)
        .await?
        .unwrap_or(user);
    if updated.role == role {
        tracing::info!(user_id = %updated.id, %role, "role synced from OIDC groups");
    } else {
        tracing::warn!(user_id = %updated.id, "not demoting the last admin");
    }
    Ok(updated)
}
