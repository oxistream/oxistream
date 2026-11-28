use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use oxistream_db::{GroupChannelOutcome, NewSubscriptionGroup, SubscriptionGroupUpdate, Uuid};
use rustypipe::validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::api::error::{ApiError, ErrorBody, ErrorCode, FieldError};
use crate::auth::CurrentUser;

pub const MAX_GROUPS: u32 = 50;
pub const MAX_NAME_CHARS: usize = 50;
const MAX_INITIAL_CHANNELS: usize = 10_000;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_groups, create_group))
        .routes(routes!(update_group, delete_group))
        .routes(routes!(move_group))
        .routes(routes!(add_channel, remove_channel))
}

/// A group of subscribed channels, e.g. "Tech" or "Music".
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionGroup {
    pub id: String,
    pub name: String,
    /// Whether the home page shows a row with the group's feed.
    pub show_on_home: bool,
    /// Subscribed channels in the group (`UC…`); a channel can be in several groups.
    pub channel_ids: Vec<String>,
}

impl From<oxistream_db::SubscriptionGroup> for SubscriptionGroup {
    fn from(g: oxistream_db::SubscriptionGroup) -> Self {
        Self {
            id: g.id.to_string(),
            name: g.name,
            show_on_home: g.show_on_home,
            channel_ids: g.channel_ids,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGroup {
    /// 1 to 50 characters, unique per user ignoring case.
    pub name: String,
    /// Default `false`.
    #[serde(default)]
    pub show_on_home: bool,
    /// Initial channels; ones the user is not subscribed to are left out.
    #[serde(default)]
    pub channel_ids: Vec<String>,
}

/// Fields to change; absent fields stay as they are.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub show_on_home: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveGroup {
    /// Zero-based target position; past the end moves the group to the end.
    pub index: u32,
}

fn not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        ErrorCode::NotFound,
        "group not found",
    )
}

/// Malformed IDs cannot exist, so they get the same 404 as unknown ones.
pub(super) fn parse_group_id(id: &str) -> Result<Uuid, ApiError> {
    Uuid::try_parse(id).map_err(|_| not_found())
}

fn check_channel_id(id: &str) -> Result<(), ApiError> {
    validate::channel_id(id).map_err(|_| ApiError::bad_request("invalid channel id"))
}

fn validate_name(name: &str, fields: &mut BTreeMap<String, FieldError>) -> String {
    let name = name.trim();
    if name.is_empty() {
        fields.insert("name".into(), FieldError::Required);
    } else if name.chars().count() > MAX_NAME_CHARS {
        fields.insert("name".into(), FieldError::TooLong);
    }
    name.to_owned()
}

fn check_fields(fields: BTreeMap<String, FieldError>) -> Result<(), ApiError> {
    if fields.is_empty() {
        Ok(())
    } else {
        Err(ApiError::validation(
            StatusCode::UNPROCESSABLE_ENTITY,
            fields,
        ))
    }
}

fn map_error(err: oxistream_db::Error) -> ApiError {
    match err {
        oxistream_db::Error::NameTaken => ApiError::validation(
            StatusCode::UNPROCESSABLE_ENTITY,
            BTreeMap::from([("name".to_owned(), FieldError::Taken)]),
        ),
        oxistream_db::Error::LimitReached => ApiError::new(
            StatusCode::CONFLICT,
            ErrorCode::LimitReached,
            format!("at most {MAX_GROUPS} groups allowed"),
        ),
        err => err.into(),
    }
}

/// The signed-in user's subscription groups, in their order.
#[utoipa::path(
    get,
    path = "/subscription-groups",
    tag = "subscriptions",
    security(("session" = [])),
    responses(
        (status = OK, body = Vec<SubscriptionGroup>),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
    )
)]
async fn list_groups(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<SubscriptionGroup>>, ApiError> {
    let groups = state.groups.list(user.id).await?;
    Ok(Json(groups.into_iter().map(Into::into).collect()))
}

/// Create a subscription group, after the existing ones.
#[utoipa::path(
    post,
    path = "/subscription-groups",
    tag = "subscriptions",
    security(("session" = [])),
    request_body = CreateGroup,
    responses(
        (status = CREATED, body = SubscriptionGroup),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = CONFLICT, description = "Too many groups", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid or taken name", body = ErrorBody),
    )
)]
async fn create_group(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    body: Result<Json<CreateGroup>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body?;
    if body.channel_ids.len() > MAX_INITIAL_CHANNELS {
        return Err(ApiError::bad_request(format!(
            "at most {MAX_INITIAL_CHANNELS} channels at once"
        )));
    }
    for id in &body.channel_ids {
        check_channel_id(id)?;
    }
    let mut fields = BTreeMap::new();
    let name = validate_name(&body.name, &mut fields);
    check_fields(fields)?;
    let group = NewSubscriptionGroup {
        name,
        show_on_home: body.show_on_home,
        channel_ids: body.channel_ids,
    };
    let created = state
        .groups
        .create(user.id, &group, MAX_GROUPS)
        .await
        .map_err(map_error)?;
    Ok((StatusCode::CREATED, Json(SubscriptionGroup::from(created))).into_response())
}

/// Rename a group or change whether it shows on the home page.
#[utoipa::path(
    patch,
    path = "/subscription-groups/{id}",
    tag = "subscriptions",
    security(("session" = [])),
    params(("id" = String, Path, description = "Group ID")),
    request_body = UpdateGroup,
    responses(
        (status = OK, body = SubscriptionGroup),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such group of the user", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid or taken name", body = ErrorBody),
    )
)]
async fn update_group(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<UpdateGroup>, JsonRejection>,
) -> Result<Json<SubscriptionGroup>, ApiError> {
    let id = parse_group_id(&id)?;
    let Json(body) = body?;
    let mut fields = BTreeMap::new();
    let update = SubscriptionGroupUpdate {
        name: body.name.map(|n| validate_name(&n, &mut fields)),
        show_on_home: body.show_on_home,
    };
    check_fields(fields)?;
    let updated = state
        .groups
        .update(user.id, id, &update)
        .await
        .map_err(map_error)?
        .ok_or_else(not_found)?;
    Ok(Json(updated.into()))
}

/// Delete a group. Its channels stay subscribed.
#[utoipa::path(
    delete,
    path = "/subscription-groups/{id}",
    tag = "subscriptions",
    security(("session" = [])),
    params(("id" = String, Path, description = "Group ID")),
    responses(
        (status = NO_CONTENT, description = "Deleted"),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such group of the user", body = ErrorBody),
    )
)]
async fn delete_group(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = parse_group_id(&id)?;
    if state.groups.delete(user.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

/// Move a group to another position.
#[utoipa::path(
    put,
    path = "/subscription-groups/{id}/position",
    tag = "subscriptions",
    security(("session" = [])),
    params(("id" = String, Path, description = "Group ID")),
    request_body = MoveGroup,
    responses(
        (status = NO_CONTENT, description = "Moved"),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such group of the user", body = ErrorBody),
    )
)]
async fn move_group(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<MoveGroup>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let id = parse_group_id(&id)?;
    let Json(body) = body?;
    if state.groups.move_to(user.id, id, body.index).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

/// Add a subscribed channel to a group. Idempotent.
#[utoipa::path(
    put,
    path = "/subscription-groups/{id}/channels/{channel_id}",
    tag = "subscriptions",
    security(("session" = [])),
    params(
        ("id" = String, Path, description = "Group ID"),
        ("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)"),
    ),
    responses(
        (status = NO_CONTENT, description = "In the group"),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such group of the user, or not subscribed to the channel", body = ErrorBody),
    )
)]
async fn add_channel(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((id, channel_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let id = parse_group_id(&id)?;
    check_channel_id(&channel_id)?;
    match state.groups.add_channel(user.id, id, &channel_id).await? {
        GroupChannelOutcome::Done => Ok(StatusCode::NO_CONTENT),
        GroupChannelOutcome::GroupNotFound => Err(not_found()),
        GroupChannelOutcome::NotSubscribed => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            ErrorCode::NotFound,
            "not subscribed to the channel",
        )),
    }
}

/// Remove a channel from a group. Succeeds if it is not in the group, too.
#[utoipa::path(
    delete,
    path = "/subscription-groups/{id}/channels/{channel_id}",
    tag = "subscriptions",
    security(("session" = [])),
    params(
        ("id" = String, Path, description = "Group ID"),
        ("channel_id" = String, Path, description = "YouTube channel ID (`UC…`)"),
    ),
    responses(
        (status = NO_CONTENT, description = "Not in the group anymore"),
        (status = BAD_REQUEST, description = "Invalid channel ID", body = ErrorBody),
        (status = UNAUTHORIZED, description = "Not signed in", body = ErrorBody),
        (status = FORBIDDEN, description = "Foreign origin", body = ErrorBody),
        (status = NOT_FOUND, description = "No such group of the user", body = ErrorBody),
    )
)]
async fn remove_channel(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((id, channel_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let id = parse_group_id(&id)?;
    check_channel_id(&channel_id)?;
    if state
        .groups
        .remove_channel(user.id, id, &channel_id)
        .await?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}
