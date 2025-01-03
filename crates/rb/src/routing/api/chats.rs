use crate::database::Database;
use crate::model::{ChatAdd, ChatNameEdit, DatabaseError};
use crate::routing::pages::PaginatedQuery;
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue};
use hcaptcha_no_wasm::Hcaptcha;
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
        .route("/from_user/{id}", post(create_request))
        // .route("/{id}", get(get_request))
        .route("/{id}/last", get(get_last_message_request))
        .route("/{id}/messages", get(get_messages_request))
        .route("/{id}/name", post(edit_name_request))
        .route("/{id}/add", post(add_friend_request))
        .route("/{id}", delete(delete_request))
        .route("/{id}/report", post(report_request))
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

/// [`Database::get_last_message_in_chat`]
pub async fn get_last_message_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    match jar.get("__Secure-Token") {
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

    Json(match database.get_last_message_in_chat(id).await {
        Ok(mut r) => {
            r.1.clean();
            DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            }
        }
        Err(e) => e.into(),
    })
}

/// [`Database::get_messages_by_chat_paginated`]
pub async fn get_messages_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Query(props): Query<PaginatedQuery>,
) -> impl IntoResponse {
    // get user from token
    match jar.get("__Secure-Token") {
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
        match database
            .get_messages_by_chat_paginated(id, props.page)
            .await
        {
            Ok(r) => {
                let mut r_clean = Vec::new();

                for mut r in r.clone() {
                    r.1.clean();
                    r_clean.push(r);
                }

                DefaultReturn {
                    success: true,
                    message: String::new(),
                    payload: Some(r_clean),
                }
            }
            Err(e) => e.into(),
        },
    )
}

/// [`Database::update_chat_name`]
pub async fn edit_name_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<ChatNameEdit>,
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
        match database
            .update_chat_name(
                ChatNameEdit {
                    chat: id,
                    name: props.name,
                },
                auth_user.id,
            )
            .await
        {
            Ok(()) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: (),
            },
            Err(e) => e.into(),
        },
    )
}

/// [`Database::add_friend`]
pub async fn add_friend_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<ChatAdd>,
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
        match database
            .add_friend_to_chat(
                ChatAdd {
                    chat: id,
                    friend: props.friend,
                },
                auth_user.id,
            )
            .await
        {
            Ok(()) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: (),
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
    headers: HeaderMap,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(req): Json<super::CreateReport>,
) -> impl IntoResponse {
    // check hcaptcha
    if let Err(e) = req
        .valid_response(&database.config.captcha.secret, None)
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

    // get real ip
    let real_ip = if let Some(ref real_ip_header) = database.config.real_ip_header {
        headers
            .get(real_ip_header.to_owned())
            .unwrap_or(&HeaderValue::from_static(""))
            .to_str()
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    // check ip
    if database.auth.get_ipban_by_ip(real_ip.clone()).await.is_ok() {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::Banned.to_string(),
            payload: (),
        });
    }

    // report
    match database
        .auth
        .create_notification(
            NotificationCreate {
                title: format!("**CHAT REPORT**: {id}"),
                content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
                address: format!("/chats/{id}"),
                recipient: "*".to_string(), // all staff
            },
            None,
        )
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
