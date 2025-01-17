use crate::database::Database;
use crate::model::{anonymous_profile, DatabaseError, QuestionCreate};
use axum::http::{HeaderMap, HeaderValue};
use authbeam::model::{IpBlockCreate, NotificationCreate};
use databeam::DefaultReturn;

use axum::response::{IntoResponse, Redirect};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use hcaptcha_no_wasm::Hcaptcha;

use axum_extra::extract::cookie::CookieJar;

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/", post(create_request))
        .route("/{id}", get(get_request))
        .route("/{id}", delete(delete_request))
        .route("/inbox/{id}/clear", post(delete_inbox_request))
        .route("/inbox/me/clear", post(delete_my_inbox_request))
        .route("/{id}/report", post(report_request))
        .route("/{id}/ipblock", post(ipblock_request))
        // ...
        .with_state(database)
}

// routes

/// [`Database::create_question`]
pub async fn create_request(
    jar: CookieJar,
    headers: HeaderMap,
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
    if database.auth.get_ipban_by_ip(real_ip.clone()).await.is_ok() {
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
            Json(match database.create_question(req, tag, real_ip).await {
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
        Json(match database.create_question(req, auth_user.id, real_ip).await {
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
        Ok(mut r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: {
                r.ip = String::new();

                // hide anonymous author id
                if r.author.id.starts_with("anonymous#") {
                    r.author.id = "anonymous".to_string()
                }

                // hide tokens, password, salt, and metadata
                r.author.clean();
                r.recipient.clean();

                // return
                Some(r)
            },
        },
        Err(e) => e.into(),
    })
}

/// Redirect to the full ID of a question through its short ID
pub async fn expand_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    match database.get_question(id).await {
        Ok(r) => Redirect::to(&format!("/@{}/q/{}", r.author.username, r.id)),
        Err(_) => Redirect::to("/"),
    }
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

/// [`Database::delete_questions_by_recipient`]
pub async fn delete_inbox_request(
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
    Json(
        match database.delete_questions_by_recipient(id, auth_user).await {
            Ok(r) => DefaultReturn {
                success: true,
                message: String::new(),
                payload: Some(r),
            },
            Err(e) => e.into(),
        },
    )
}

/// [`Database::delete_questions_by_recipient`]
pub async fn delete_my_inbox_request(
    jar: CookieJar,
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
    Json(
        match database
            .delete_questions_by_recipient(auth_user.id.clone(), auth_user)
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

/// Report a question
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

    // get question
    if let Err(_) = database.get_question(id.clone()).await {
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
                title: format!("**QUESTION REPORT**: {id}"),
                content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
                address: format!("/question/{id}"),
                recipient: "*".to_string(), // all staff
            },
            None,
        )
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Question reported!".to_string(),
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

/// IP block a question's author
pub async fn ipblock_request(
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

    // get question
    let question = match database.get_question(id.clone()).await {
        Ok(q) => q,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
        }
    };

    // block
    match database
        .auth
        .create_ipblock(
            IpBlockCreate {
                ip: question.ip,
                context: question.content,
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
