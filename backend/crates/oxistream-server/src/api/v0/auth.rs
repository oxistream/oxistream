use std::collections::BTreeMap;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use oxistream_db::{NewUser, Role, User};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, ErrorCode};
use crate::auth::password::{PASSWORD_MAX_LENGTH, validate_password, validate_username};
use crate::auth::{ClientAddr, CurrentUser, cookie};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(register))
        .routes(routes!(login))
        .routes(routes!(logout))
        .routes(routes!(me))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct Credentials {
    /// 3 to 32 letters, digits, dots, dashes or underscores. Case-insensitive for sign-in.
    pub username: String,
    /// 12 to 256 characters.
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    User,
    Admin,
}

impl From<Role> for UserRole {
    fn from(role: Role) -> Self {
        match role {
            Role::User => Self::User,
            Role::Admin => Self::Admin,
        }
    }
}

/// The signed-in account.
#[derive(Debug, Serialize, ToSchema)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl From<User> for Account {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: user.role.into(),
            created_at: user.created_at,
        }
    }
}

fn local_auth_disabled() -> ApiError {
    ApiError::new(
        StatusCode::FORBIDDEN,
        ErrorCode::LocalAuthDisabled,
        "local accounts are disabled on this instance",
    )
}

fn signed_in(status: StatusCode, cookie: axum::http::HeaderValue, user: User) -> Response {
    (
        status,
        [(header::SET_COOKIE, cookie)],
        Json(Account::from(user)),
    )
        .into_response()
}

/// Create a local account and sign in. The first account of an instance becomes an admin.
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = Credentials,
    responses(
        (status = CREATED, description = "Account created, session cookie set", body = Account,
            headers(("set-cookie" = String, description = "`oxistream_session` cookie"))),
        (status = BAD_REQUEST, description = "Malformed body", body = ErrorBody),
        (status = FORBIDDEN, description = "Local accounts or registration disabled, or foreign origin", body = ErrorBody),
        (status = CONFLICT, description = "Username taken (`fields.username = taken`)", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid username or password, see `fields`", body = ErrorBody),
        (status = TOO_MANY_REQUESTS, description = "Too many attempts", body = ErrorBody),
    )
)]
async fn register(
    State(state): State<AppState>,
    ClientAddr(ip): ClientAddr,
    headers: HeaderMap,
    body: Result<Json<Credentials>, JsonRejection>,
) -> Result<Response, ApiError> {
    let auth = &state.auth;
    if !auth.config().local_enabled {
        return Err(local_auth_disabled());
    }
    if !auth.config().registration_enabled {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            ErrorCode::RegistrationDisabled,
            "registration is disabled on this instance",
        ));
    }
    let Json(Credentials { username, password }) = body?;
    let username = username.trim().to_owned();

    let mut fields = BTreeMap::new();
    if let Err(err) = validate_username(&username) {
        fields.insert("username".to_owned(), err);
    }
    if let Err(err) = validate_password(&password) {
        fields.insert("password".to_owned(), err);
    }
    if !fields.is_empty() {
        return Err(ApiError::validation(
            StatusCode::UNPROCESSABLE_ENTITY,
            fields,
        ));
    }
    if !auth.allow_attempt(ip, None) {
        return Err(ApiError::too_many_requests());
    }

    let password_hash = auth.hasher().hash(password).await?;
    let user = match auth
        .users()
        .create(&NewUser {
            username,
            password_hash: Some(password_hash),
        })
        .await
    {
        Ok(user) => user,
        Err(oxistream_db::Error::UsernameTaken) => {
            return Err(ApiError::validation(
                StatusCode::CONFLICT,
                BTreeMap::from([("username".to_owned(), crate::api::error::FieldError::Taken)]),
            ));
        }
        Err(err) => return Err(err.into()),
    };
    tracing::info!(user_id = %user.id, role = %user.role, "account registered");
    let cookie = auth.start_session(&user, &headers, ip).await?;
    Ok(signed_in(StatusCode::CREATED, cookie, user))
}

/// Sign in with a local account.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = Credentials,
    responses(
        (status = OK, description = "Signed in, session cookie set", body = Account,
            headers(("set-cookie" = String, description = "`oxistream_session` cookie"))),
        (status = BAD_REQUEST, description = "Malformed body", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Unknown username or wrong password", body = ErrorBody),
        (status = FORBIDDEN, description = "Account disabled, local accounts disabled, or foreign origin", body = ErrorBody),
        (status = TOO_MANY_REQUESTS, description = "Too many attempts", body = ErrorBody),
    )
)]
async fn login(
    State(state): State<AppState>,
    ClientAddr(ip): ClientAddr,
    headers: HeaderMap,
    body: Result<Json<Credentials>, JsonRejection>,
) -> Result<Response, ApiError> {
    let auth = &state.auth;
    if !auth.config().local_enabled {
        return Err(local_auth_disabled());
    }
    let Json(Credentials { username, password }) = body?;
    let username = username.trim();
    if !auth.allow_attempt(ip, Some(username)) {
        return Err(ApiError::too_many_requests());
    }
    let invalid = || {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::InvalidCredentials,
            "invalid username or password",
        )
    };
    if username.is_empty() || password.is_empty() || password.chars().count() > PASSWORD_MAX_LENGTH
    {
        return Err(invalid());
    }

    let credentials = auth.users().find_by_username(username).await?;
    let hash = credentials.as_ref().and_then(|c| c.password_hash.clone());
    let verified = auth.hasher().verify(password, hash).await?;
    let user = match credentials {
        Some(credentials) if verified => credentials.user,
        _ => return Err(invalid()),
    };
    if user.disabled {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            ErrorCode::AccountDisabled,
            "this account is disabled",
        ));
    }
    let cookie = auth.start_session(&user, &headers, ip).await?;
    Ok(signed_in(StatusCode::OK, cookie, user))
}

/// End the current session. Succeeds without a session, too.
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = NO_CONTENT, description = "Signed out, session cookie cleared",
            headers(("set-cookie" = String, description = "Expired `oxistream_session` cookie"))),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
    )
)]
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    if let Some(hash) = cookie::session_token(&headers).and_then(cookie::hash_token) {
        state.auth.sessions().delete(&hash).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, state.auth.clear_cookie())],
    )
        .into_response())
}

/// The signed-in account.
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(("session" = [])),
    responses(
        (status = OK, body = Account),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn me(CurrentUser(user): CurrentUser) -> Json<Account> {
    Json(user.into())
}
