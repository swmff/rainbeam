use crate::database::Database;
use crate::model::{DatabaseError, ResponseCreate, ResponseEdit, ResponseEditTags};
use axum::routing::put;
use hcaptcha::Hcaptcha;
use xsu_authman::model::{NotificationCreate, ProfileMetadata};
use xsu_dataman::DefaultReturn;

use axum::response::{IntoResponse, Redirect};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};

use axum_extra::extract::cookie::CookieJar;

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/", post(create_request))
        .route("/:id", get(get_request))
        .route("/:id", put(edit_request))
        .route("/:id/tags", put(edit_tags_request))
        .route("/:id", delete(delete_request))
        .route("/:id/report", post(report_request))
        // ...
        .with_state(database)
}

// routes

/// [`Database::create_response`]
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(req): Json<ResponseCreate>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua.id,
            Err(_) => return Json(DatabaseError::NotAllowed.into()),
        },
        None => return Json(DatabaseError::NotAllowed.into()),
    };

    // ...
    Json(match database.create_response(req, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::get_response`]
pub async fn get_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    Json(match database.get_response(id).await {
        Ok(mut r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: {
                // hide anonymous author id
                if r.0.author.id.starts_with("anonymous#") {
                    r.0.author.id = "anonymous".to_string()
                }

                // hide tokens, password, salt, and metadata
                r.0.author.password = String::new();
                r.0.author.salt = String::new();
                r.0.author.tokens = Vec::new();
                r.0.author.metadata = ProfileMetadata::default();

                r.0.recipient.password = String::new();
                r.0.recipient.salt = String::new();
                r.0.recipient.tokens = Vec::new();
                r.0.recipient.metadata = ProfileMetadata::default();

                r.1.author.password = String::new();
                r.1.author.salt = String::new();
                r.1.author.tokens = Vec::new();
                r.1.author.metadata = ProfileMetadata::default();

                // return
                Some(r)
            },
        },
        Err(e) => e.into(),
    })
}

/// Redirect to the full ID of a response through its short ID
pub async fn expand_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    match database.get_response(id).await {
        Ok(r) => Redirect::to(&format!("/response/{}", r.1.id)),
        Err(_) => Redirect::to("/"),
    }
}

/// [`Database::update_response_content`]
pub async fn edit_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(req): Json<ResponseEdit>,
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
    Json(
        match database
            .update_response_content(id, req.content, auth_user)
            .await
        {
            Ok(r) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            },
            Err(e) => e.into(),
        },
    )
}

/// [`Database::update_response_tags`]
pub async fn edit_tags_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(req): Json<ResponseEditTags>,
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
    Json(
        match database.update_response_tags(id, req.tags, auth_user).await {
            Ok(r) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            },
            Err(e) => e.into(),
        },
    )
}

/// [`Database::delete_response`]
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
    Json(match database.delete_response(id, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// Report a response
pub async fn report_request(
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(req): Json<super::CreateReport>,
) -> impl IntoResponse {
    // check hcaptcha
    if let Err(e) = req
        .valid_response(&database.server_options.captcha.secret, None)
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: (),
        });
    }

    // get response
    if let Err(_) = database.get_response(id.clone()).await {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotFound.to_string(),
            payload: (),
        });
    };

    match database
        .auth
        .create_notification(NotificationCreate {
            title: format!("**RESPONSE REPORT**: {id}"),
            content: req.content,
            address: format!("/response/{id}"),
            recipient: "*".to_string(), // all staff
        })
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Response reported!".to_string(),
                payload: (),
            })
        }
        Err(_) => Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotFound.to_string(),
            payload: (),
        }),
    }
}
