use crate::database::Database;
use crate::model::{DatabaseError, MessageCreate, ResponseEdit};
use axum::http::{HeaderMap, HeaderValue};
use hcaptcha_no_wasm::Hcaptcha;
use authbeam::model::NotificationCreate;
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};

use axum_extra::extract::cookie::CookieJar;

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/", post(create_request))
        .route("/{id}", get(get_request))
        .route("/{id}", put(edit_request))
        .route("/{id}", delete(delete_request))
        .route("/{id}/report", post(report_request))
        // ...
        .with_state(database)
}

// routes

/// [`Database::create_message`]
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(req): Json<MessageCreate>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Json(DatabaseError::NotAllowed.into()),
        },
        None => return Json(DatabaseError::NotAllowed.into()),
    };

    // ...
    Json(match database.create_message(req, auth_user.id).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::get_message`]
pub async fn get_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    Json(match database.get_message(id).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r.0),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::edit_message`]
pub async fn edit_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<ResponseEdit>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
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
        match database.edit_message(id, props.content, auth_user).await {
            Ok(r) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            },
            Err(e) => e.into(),
        },
    )
}

/// [`Database::delete_message`]
pub async fn delete_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
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
    Json(match database.delete_message(id, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// Report a message
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

    // get message
    if let Err(_) = database.get_message(id.clone()).await {
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
    if database.auth.get_ipban_by_ip(&real_ip).await.is_ok() {
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
                title: format!("**MESSAGE REPORT**: {id}"),
                content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
                address: format!("/message/{id}"),
                recipient: "*".to_string(), // all staff
            },
            None,
        )
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Message reported!".to_string(),
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
