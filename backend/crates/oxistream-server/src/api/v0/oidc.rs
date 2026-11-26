use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use oxistream_db::{Role, User};
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::auth::ClientAddr;
use crate::auth::oidc::{
    CallbackParams, LoginError, VerifiedLogin, claims, pending, safe_redirect,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(oidc_login))
        .routes(routes!(oidc_callback))
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
