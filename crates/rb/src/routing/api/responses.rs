use crate::database::Database;
use crate::model::{DatabaseError, ResponseCreate, ResponseEdit, ResponseEditTags};
use axum::http::{HeaderMap, HeaderValue};
use axum::routing::put;
use hcaptcha::Hcaptcha;
use authbeam::model::NotificationCreate;
use databeam::DefaultReturn;
use citrus_client::model::CitrusID;

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
        .route("/:id/unsend", post(unsend_request))
        .route("/:id/report", post(report_request))
        // timelines
        .route("/timeline/home", get(home_timeline_request))
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
                r.1.id = CitrusID::new(&database.server_options.citrus_id, &r.1.id).0;

                // hide anonymous author id
                if r.0.author.id.starts_with("anonymous#") {
                    r.0.author.id = "anonymous".to_string()
                }

                // hide tokens, password, salt, and metadata
                r.0.author.clean();
                r.0.recipient.clean();
                r.1.author.clean();

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
    Json(match database.delete_response(id, auth_user, false).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::unsend_response`]
pub async fn unsend_request(
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
    Json(match database.unsend_response(id, auth_user).await {
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
    headers: HeaderMap,
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

    // get real ip
    let real_ip = if let Some(ref real_ip_header) = database.server_options.real_ip_header {
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
                title: format!("**RESPONSE REPORT**: {id}"),
                content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
                address: format!("/response/{id}"),
                recipient: "*".to_string(), // all staff
            },
            None,
        )
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

/// Home timeline request ("/")
pub async fn home_timeline_request(
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
            .get_responses_by_following(auth_user.id.to_owned())
            .await
        {
            Ok(mut r) => {
                for response in &mut r {
                    response.1.author.clean();
                    response.0.recipient.clean();
                    response.0.author.clean();
                }

                DefaultReturn {
                    success: true,
                    message: String::new(),
                    payload: Some(r),
                }
            }
            Err(e) => e.into(),
        },
    )
}
