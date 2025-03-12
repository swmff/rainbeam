use crate::database::Database;
use crate::model::{anonymous_profile, CommentCreate, DatabaseError, ResponseEdit};
use axum::http::{HeaderMap, HeaderValue};
use hcaptcha_no_wasm::Hcaptcha;
use authbeam::model::{IpBlockCreate, NotificationCreate};
use databeam::prelude::DefaultReturn;

use axum::response::{IntoResponse, Redirect};
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
        .route("/{id}/ipblock", post(ipblock_request))
        // ...
        .with_state(database)
}

// routes

/// [`Database::create_comment`]
pub async fn create_request(
    jar: CookieJar,
    headers: HeaderMap,
    State(database): State<Database>,
    Json(req): Json<CommentCreate>,
) -> impl IntoResponse {
    // get user from token
    let mut was_not_anonymous = false;

    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => {
                was_not_anonymous = true;
                ua
            }
            Err(_) => anonymous_profile(database.create_anonymous().0),
        },
        None => anonymous_profile(database.create_anonymous().0),
    };

    let existing_tag = match jar.get("__Secure-Question-Tag") {
        Some(c) => c.value_trimmed().to_string(),
        None => String::new(),
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
        return (
            [
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Set-Cookie".to_string(), String::new()),
            ],
            Json(DefaultReturn {
                success: false,
                message: DatabaseError::Banned.to_string(),
                payload: None,
            }),
        );
    }

    // get correct username
    let use_anonymous_anyways = req.anonymous; // this is the "Hide your name" field

    if (auth_user.username == "anonymous") | use_anonymous_anyways {
        let tag = if was_not_anonymous && use_anonymous_anyways {
            // use real username as tag
            format!("anonymous#{}", auth_user.id)
        } else if !existing_tag.is_empty() {
            // use existing tag
            existing_tag
        } else if !was_not_anonymous {
            // use id as tag
            auth_user.id
        } else {
            // use id as tag
            if auth_user.username == "anonymous" {
                auth_user.id
            } else {
                auth_user.id
            }
        };

        // create as anonymous
        return (
            [
                ("Content-Type".to_string(), "text/plain".to_string()),
                (
                    "Set-Cookie".to_string(),
                    format!(
                        "__Secure-Question-Tag={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
                        tag,
                        60 * 60 * 24 * 365
                    ),
                ),
            ],
            Json(match database.create_comment(req, tag, real_ip).await {
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
                    auth_user.username.replace("anonymous#", ""),
                    60 * 60 * 24 * 365
                ),
            ),
        ],
        Json(match database.create_comment(req, auth_user.id, real_ip).await {
            Ok(r) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            },
            Err(e) => e.into(),
        })
    )
}

/// [`Database::get_comment`]
pub async fn get_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    Json(match database.get_comment(id, true).await {
        Ok(mut r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: {
                r.0.author.clean();
                Some(r)
            },
        },
        Err(e) => e.into(),
    })
}

/// Redirect to the full ID of a comment through its short ID
pub async fn expand_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    match database.get_comment(id, false).await {
        Ok(c) => Redirect::to(&format!("/@{}/c/{}", c.0.author.username, c.0.id)),
        Err(_) => Redirect::to("/"),
    }
}

/// [`Database::update_comment_content`]
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
        match database
            .update_comment_content(id, req.content, auth_user)
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

/// [`Database::delete_comment`]
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
    Json(match database.delete_comment(id, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// Report a comment
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

    // get comment
    if let Err(_) = database.get_comment(id.clone(), false).await {
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
                title: format!("**COMMENT REPORT**: {id}"),
                content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
                address: format!("/comment/{id}"),
                recipient: "*".to_string(), // all staff
            },
            None,
        )
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Comment reported!".to_string(),
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

/// IP block a comment's author
pub async fn ipblock_request(
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

    // get comment
    let comment = match database.get_comment(id.clone(), false).await {
        Ok(q) => q.0,
        Err(e) => return Json(e.to_json()),
    };

    // block
    match database
        .auth
        .create_ipblock(
            IpBlockCreate {
                ip: comment.ip,
                context: comment.content,
            },
            auth_user,
        )
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "IP blocked!".to_string(),
                payload: (),
            })
        }
        Err(_) => Json(DefaultReturn {
            success: false,
            message: DatabaseError::Other.to_string(),
            payload: (),
        }),
    }
}
