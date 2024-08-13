use crate::database::Database;
use crate::model::{DatabaseError, QuestionCreate};
use xsu_dataman::DefaultReturn;

use axum::response::IntoResponse;
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
        .route("/:id", delete(delete_request))
        // ...
        .with_state(database)
}

// routes

/// [`Database::create_question`]
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(req): Json<QuestionCreate>,
) -> impl IntoResponse {
    // get user from token
    let mut was_not_anonymous = false;

    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => {
                was_not_anonymous = true;
                ua.username
            }
            Err(_) => String::from("anonymous"),
        },
        None => String::from("anonymous"),
    };

    let existing_tag = match jar.get("__Secure-Question-Tag") {
        Some(c) => c.value_trimmed().to_string(),
        None => String::new(),
    };

    // get correct username
    let use_anonymous_anyways = req.anonymous; // this is the "Hide your name" field

    let username = if use_anonymous_anyways {
        if !existing_tag.is_empty() {
            // this will use the tag we already got when creating this question
            format!("anonymous#{existing_tag}")
        } else {
            // this will make us generate a new tag and send it as a cookie
            "anonymous".to_string()
        }
    } else {
        auth_user.clone()
    };

    if username == "anonymous" {
        // add tag
        let tag = if was_not_anonymous {
            // use the user's real username as their tag
            // this allows us to detect authenticated users who are breaking rules as anonymous
            (format!("anonymous#{auth_user}"), auth_user)
        } else {
            // create a random tag
            database.create_anonymous()
        };

        // create as anonymous
        return (
            [
                ("Content-Type".to_string(), "text/plain".to_string()),
                (
                    "Set-Cookie".to_string(),
                    format!(
                        "__Secure-Question-Tag={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
                        tag.1,
                        60 * 60 * 24 * 365
                    ),
                ),
            ],
            Json(match database.create_question(req, username).await {
                Ok(r) => DefaultReturn {
                    success: true,
                    message: String::new(),
                    payload: Some(r),
                },
                Err(e) => e.into(),
            }),
        );
    }

    // ...
    (
        [
            ("Content-Type".to_string(), "text/plain".to_string()),
            (
                "Set-Cookie".to_string(),
                format!(
                    "__Secure-Question-Tag={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
                    username.replace("anonymous#", ""),
                    60 * 60 * 24 * 365
                ),
            ),
        ],
        Json(match database.create_question(req, username).await {
            Ok(r) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            },
            Err(e) => e.into(),
        })
    )
}

/// [`Database::get_question`]
pub async fn get_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    Json(match database.get_question(id).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::delete_question`]
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
    Json(match database.delete_question(id, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}
