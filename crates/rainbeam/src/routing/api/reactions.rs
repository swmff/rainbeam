use crate::database::Database;
use crate::model::{AssetType, DatabaseError, ReactionCreate};
use authbeam::model::NotificationCreate;
use databeam::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};

use axum_extra::extract::cookie::CookieJar;

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/:id", post(create_request))
        .route("/:id", get(get_request))
        .route("/:id", delete(delete_request))
        // ...
        .with_state(database)
}

// routes

/// [`Database::create_reaction`]
pub async fn create_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<ReactionCreate>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Json(DatabaseError::NotAllowed.into()),
        },
        None => return Json(DatabaseError::NotAllowed.into()),
    };

    // verify asset from type
    match props.r#type {
        AssetType::Question => {
            let asset = match database.get_question(id.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    })
                }
            };

            // create notification
            if let Err(_) = database
                .auth
                .create_notification(NotificationCreate {
                    title: format!(
                        "[@{}](/+u/{}) has reacted to a question you created!",
                        auth_user.username, auth_user.id
                    ),
                    content: String::new(),
                    address: format!("/question/{id}"),
                    recipient: asset.author.id,
                })
                .await
            {
                return Json(DefaultReturn {
                    success: false,
                    message: DatabaseError::Other.to_string(),
                    payload: None,
                });
            }
        }
        AssetType::Response => {
            let asset = match database.get_response(id.clone(), false).await {
                Ok(r) => r.1,
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    })
                }
            };

            // create notification
            if let Err(_) = database
                .auth
                .create_notification(NotificationCreate {
                    title: format!(
                        "[@{}](/+u/{}) has reacted to a response you created!",
                        auth_user.username, auth_user.id
                    ),
                    content: String::new(),
                    address: format!("/response/{id}"),
                    recipient: asset.author.id,
                })
                .await
            {
                return Json(DefaultReturn {
                    success: false,
                    message: DatabaseError::Other.to_string(),
                    payload: None,
                });
            }
        }
        AssetType::Comment => {
            let asset = match database.get_comment(id.clone(), false).await {
                Ok(r) => r.0,
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    })
                }
            };

            // create notification
            if let Err(_) = database
                .auth
                .create_notification(NotificationCreate {
                    title: format!(
                        "[@{}](/+u/{}) has reacted to a comment you created!",
                        auth_user.username, auth_user.id
                    ),
                    content: String::new(),
                    address: format!("/comment/{id}"),
                    recipient: asset.author.id,
                })
                .await
            {
                return Json(DefaultReturn {
                    success: false,
                    message: DatabaseError::Other.to_string(),
                    payload: None,
                });
            }
        }
    };

    // ...
    Json(match database.create_reaction(id, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::get_reaction`]
pub async fn get_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(_) => {
                return Json(DatabaseError::NotAllowed.into());
            }
        },
        None => {
            return Json(DatabaseError::NotAllowed.into());
        }
    };

    Json(match database.get_reaction(auth_user.id, id).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::delete_reaction`]
pub async fn delete_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(_) => {
                return Json(DatabaseError::NotAllowed.into());
            }
        },
        None => {
            return Json(DatabaseError::NotAllowed.into());
        }
    };

    // ...
    Json(match database.delete_reaction(id, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}
