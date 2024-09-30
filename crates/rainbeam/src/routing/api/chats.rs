use crate::database::Database;
use crate::model::DatabaseError;
use hcaptcha::Hcaptcha;
use authbeam::model::NotificationCreate;
use databeam::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    routing::{delete, post},
    Json, Router,
};

use axum_extra::extract::cookie::CookieJar;

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/from_user/:id", post(create_request))
        // .route("/:id", get(get_request))
        .route("/:id", delete(delete_request))
        .route("/:id/report", post(report_request))
        // ...
        .with_state(database)
}

// routes

/// [`Database::create_chat`]
pub async fn create_request(
    jar: CookieJar,
    Path(other_user_id): Path<String>,
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
            Err(_) => return Json(DatabaseError::NotAllowed.into()),
        },
        None => return Json(DatabaseError::NotAllowed.into()),
    };

    Json(
        match database.create_chat(auth_user.id, other_user_id).await {
            Ok(r) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            },
            Err(e) => e.into(),
        },
    )
}

/// [`Database::leave_chat`]
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
    Json(match database.leave_chat(id, auth_user.id).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// Report a chat
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

    // get chat
    if let Err(_) = database.get_chat(id.clone()).await {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotFound.to_string(),
            payload: (),
        });
    };

    match database
        .auth
        .create_notification(NotificationCreate {
            title: format!("**CHAT REPORT**: {id}"),
            content: req.content,
            address: format!("/chat/{id}"),
            recipient: "*".to_string(), // all staff
        })
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Chat reported!".to_string(),
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
