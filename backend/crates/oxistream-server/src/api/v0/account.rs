use std::collections::BTreeMap;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use oxistream_db::DeleteOutcome;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::local_auth_disabled;
use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, ErrorCode, FieldError};
use crate::auth::password::{PASSWORD_MAX_LENGTH, validate_password};
use crate::auth::{ClientAddr, CurrentUser, cookie};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(change_password))
        .routes(routes!(delete_account))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PasswordChange {
    pub current_password: String,
    /// 12 to 256 characters.
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AccountDeletion {
    /// The account's username, typed as confirmation; compared ignoring case.
    pub username: String,
    /// The current password; required for accounts that have one.
    #[serde(default)]
    pub password: Option<String>,
}

fn no_password() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        ErrorCode::NoPassword,
        "this account signs in via single sign-on and has no password",
    )
}

fn mismatch(field: &str) -> ApiError {
    ApiError::validation(
        StatusCode::UNPROCESSABLE_ENTITY,
        BTreeMap::from([(field.to_owned(), FieldError::Mismatch)]),
    )
}

async fn verify_password(
    state: &AppState,
    user_id: oxistream_db::Uuid,
    password: String,
) -> Result<bool, ApiError> {
    if password.is_empty() || password.chars().count() > PASSWORD_MAX_LENGTH {
        return Ok(false);
    }
    let hash = state.auth.users().password_hash(user_id).await?;
    Ok(state.auth.hasher().verify(password, hash).await?)
}

/// Change the password of a local account. Ends the account's other sessions.
#[utoipa::path(
    post,
    path = "/account/password",
    tag = "account",
    security(("session" = [])),
    request_body = PasswordChange,
    responses(
        (status = NO_CONTENT, description = "Password changed"),
        (status = BAD_REQUEST, description = "Malformed body", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Local accounts disabled, or foreign origin", body = ErrorBody),
        (status = CONFLICT, description = "Single sign-on account without password (`no_password`)", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Wrong current password (`fields.current_password = mismatch`) or invalid new password", body = ErrorBody),
        (status = TOO_MANY_REQUESTS, description = "Too many attempts", body = ErrorBody),
    )
)]
async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ClientAddr(ip): ClientAddr,
    headers: HeaderMap,
    body: Result<Json<PasswordChange>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let auth = &state.auth;
    if !auth.config().local_enabled {
        return Err(local_auth_disabled());
    }
    if !user.has_password {
        return Err(no_password());
    }
    let Json(PasswordChange {
        current_password,
        new_password,
    }) = body?;
    if let Err(err) = validate_password(&new_password) {
        return Err(ApiError::validation(
            StatusCode::UNPROCESSABLE_ENTITY,
            BTreeMap::from([("new_password".to_owned(), err)]),
        ));
    }
    if !auth.allow_attempt(ip, Some(&user.username)) {
        return Err(ApiError::too_many_requests());
    }
    if !verify_password(&state, user.id, current_password).await? {
        return Err(mismatch("current_password"));
    }
    let hash = auth.hasher().hash(new_password).await?;
    if !auth.users().set_password(user.id, &hash).await? {
        return Err(no_password());
    }
    if let Some(current) = cookie::session_token(&headers).and_then(cookie::hash_token) {
        auth.sessions().delete_others(user.id, &current).await?;
    }
    tracing::info!(user_id = %user.id, "password changed");
    Ok(StatusCode::NO_CONTENT)
}

/// Delete the signed-in account with everything it owns: subscriptions, playlists, history,
/// settings and sessions. The last admin can only delete their account once no other users
/// remain. Single sign-on users get a new, empty account on their next sign-in.
#[utoipa::path(
    delete,
    path = "/account",
    tag = "account",
    security(("session" = [])),
    request_body = AccountDeletion,
    responses(
        (status = NO_CONTENT, description = "Account deleted, session cookie cleared",
            headers(("set-cookie" = String, description = "Expired `oxistream_session` cookie"))),
        (status = BAD_REQUEST, description = "Malformed body", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = CONFLICT, description = "Last admin while other users remain (`last_admin`)", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Username or password does not match (`fields`)", body = ErrorBody),
        (status = TOO_MANY_REQUESTS, description = "Too many attempts", body = ErrorBody),
    )
)]
async fn delete_account(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ClientAddr(ip): ClientAddr,
    body: Result<Json<AccountDeletion>, JsonRejection>,
) -> Result<Response, ApiError> {
    let auth = &state.auth;
    let Json(AccountDeletion { username, password }) = body?;
    if !username.trim().eq_ignore_ascii_case(&user.username) {
        return Err(mismatch("username"));
    }
    if user.has_password {
        if !auth.allow_attempt(ip, Some(&user.username)) {
            return Err(ApiError::too_many_requests());
        }
        if !verify_password(&state, user.id, password.unwrap_or_default()).await? {
            return Err(mismatch("password"));
        }
    }
    match auth.users().delete(user.id).await? {
        DeleteOutcome::Deleted | DeleteOutcome::NotFound => {}
        DeleteOutcome::LastAdmin => {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                ErrorCode::LastAdmin,
                "the last admin cannot delete their account while other users remain",
            ));
        }
    }
    tracing::info!(user_id = %user.id, "account deleted");
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, auth.clear_cookie())],
    )
        .into_response())
}
