use crate::database::Database;
use crate::model::{CircleCreate, DatabaseError, EditCircleMetadata, MembershipStatus};
use axum::http::{HeaderMap, HeaderValue};
use hcaptcha_no_wasm::Hcaptcha;
use authbeam::model::NotificationCreate;
use databeam::DefaultReturn;
use pathbufd::pathd;

use axum::response::{IntoResponse, Redirect};
use axum::{
    body::Body,
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};

use authbeam::api::profile::read_image;
use axum_extra::extract::cookie::CookieJar;

pub fn routes(database: Database) -> Router {
    Router::new()
        // global
        .route("/", post(create_request))
        .route("/{id}", get(get_request))
        // specific
        // .route("/{name}", put(edit_request))
        .route("/{id}/metadata", post(edit_metadata_request))
        .route("/{id}", delete(delete_request))
        .route("/{id}/report", post(report_request))
        // members
        .route("/{id}/accept_invite", post(accept_invite_request))
        .route("/{id}/invite/{username}", post(send_invite_request))
        .route("/{id}/kick/{username}", post(kick_member_request))
        .route("/{id}/leave", post(leave_request))
        // as a profile
        .route("/{id}/avatar", get(avatar_request))
        .route("/{id}/banner", get(banner_request))
        // ...
        .with_state(database)
}

// routes

/// Redirect an ID to a full username
pub async fn expand_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    match database.get_circle(id).await {
        Ok(r) => Redirect::to(&format!("/+{}", r.name)),
        Err(_) => Redirect::to("/"),
    }
}

/// [`Database::create_circle`]
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(req): Json<CircleCreate>,
) -> impl IntoResponse {
    // check hcaptcha
    if let Err(e) = req
        .valid_response(&database.config.captcha.secret, None)
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

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
    Json(match database.create_circle(req, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// [`Database::get_circle`]
pub async fn get_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    Json(match database.get_circle(id).await {
        Ok(mut r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: {
                r.owner.clean();
                Some(r)
            },
        },
        Err(e) => e.into(),
    })
}

/// [`Database::update_circle_metadata`]
pub async fn edit_metadata_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(req): Json<EditCircleMetadata>,
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
            .update_circle_metadata(id, req.metadata, auth_user)
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

/// Accept an invite to a circle
pub async fn accept_invite_request(
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

    // make sure we have a pending invite
    let current_status = database
        .get_user_circle_membership(auth_user.id.clone(), id.clone())
        .await;

    if current_status != MembershipStatus::Pending {
        return Json(DatabaseError::NotAllowed.into());
    }

    // ...
    Json(
        match database
            .set_user_circle_membership(auth_user.id, id, MembershipStatus::Active, false)
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

/// Send an invite to a circle
pub async fn send_invite_request(
    jar: CookieJar,
    Path((id, user)): Path<(String, String)>,
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

    // get circle
    let circle = match database.get_circle(id.clone()).await {
        Ok(c) => c,
        Err(e) => return Json(e.into()),
    };

    if auth_user.id != circle.owner.id {
        return Json(DatabaseError::NotAllowed.into());
    }

    // get user
    let user = match database.get_profile(user).await {
        Ok(ua) => ua,
        Err(e) => return Json(e.into()),
    };

    // ...
    Json(
        match database
            .set_user_circle_membership(user.id, id, MembershipStatus::Pending, false)
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

/// Kick a member from the circle
pub async fn kick_member_request(
    jar: CookieJar,
    Path((id, user)): Path<(String, String)>,
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

    // get circle
    let circle = match database.get_circle(id.clone()).await {
        Ok(c) => c,
        Err(e) => return Json(e.into()),
    };

    if auth_user.id != circle.owner.id {
        return Json(DatabaseError::NotAllowed.into());
    }

    // get user
    let user = match database.get_profile(user).await {
        Ok(ua) => ua,
        Err(e) => return Json(e.into()),
    };

    if user.id == circle.owner.id {
        return Json(DatabaseError::NotAllowed.into());
    }

    // ...
    Json(
        match database
            .set_user_circle_membership(user.id, id, MembershipStatus::Inactive, false)
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

/// Leave a the circle
pub async fn leave_request(
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

    // get circle
    let circle = match database.get_circle(id.clone()).await {
        Ok(c) => c,
        Err(e) => return Json(e.into()),
    };

    // get user
    let user = auth_user;

    if user.id == circle.owner.id {
        return Json(DatabaseError::NotAllowed.into());
    }

    // ...
    Json(
        match database
            .set_user_circle_membership(user.id, id, MembershipStatus::Inactive, false)
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

/// [`Database::delete_circle`]
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
    Json(match database.delete_circle(id, auth_user).await {
        Ok(r) => DefaultReturn {
            success: true,
            message: String::new(),
            payload: Some(r),
        },
        Err(e) => e.into(),
    })
}

/// Report a circle
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

    // get response
    let circle = match database.get_circle(id.clone()).await {
        Ok(c) => c,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
        }
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
                title: format!("**CIRCLE REPORT**: [/+{}](/+{})", circle.name, circle.name),
                content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
                address: format!("/+{}", circle.name),
                recipient: "*".to_string(), // all staff
            },
            None,
        )
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Circle reported!".to_string(),
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

/// Get a circle's avatar image
pub async fn avatar_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let circle = match database.get_circle(id).await {
        Ok(ua) => ua,
        Err(_) => {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    pathd!("{}/images", database.config.static_dir),
                    "default-avatar.svg".to_string(),
                )),
            );
        }
    };

    // ...
    let avatar_url = match circle.metadata.kv.get("sparkler:avatar_url") {
        Some(r) => r,
        None => "",
    };

    if avatar_url.starts_with(&database.config.host) {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                pathd!("{}/images", database.config.static_dir),
                "default-avatar.svg".to_string(),
            )),
        );
    }

    for host in database.config.blocked_hosts {
        if avatar_url.starts_with(&host) {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    pathd!("{}/images", database.config.static_dir),
                    "default-avatar.svg".to_string(),
                )),
            );
        }
    }

    // get profile image
    if avatar_url.is_empty() {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                pathd!("{}/images", database.config.static_dir),
                "default-avatar.svg".to_string(),
            )),
        );
    }

    let guessed_mime = mime_guess::from_path(avatar_url)
        .first_raw()
        .unwrap_or("application/octet-stream");

    match database.auth.http.get(avatar_url).send().await {
        Ok(stream) => {
            if let Some(ct) = stream.headers().get("Content-Type") {
                if !ct.to_str().unwrap().starts_with("image/") {
                    // if we failed to load the image, we might get back text/html or something
                    // we're going to return the default image if we got something that isn't
                    // an image (or has an incorrect mime)
                    return (
                        [("Content-Type", "image/svg+xml")],
                        Body::from(read_image(
                            pathd!("{}/images", database.config.static_dir),
                            "default-avatar.svg".to_string(),
                        )),
                    );
                }
            }

            (
                [(
                    "Content-Type",
                    if guessed_mime == "text/html" {
                        "text/plain"
                    } else {
                        guessed_mime
                    },
                )],
                Body::from_stream(stream.bytes_stream()),
            )
        }
        Err(_) => (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                pathd!("{}/images", database.config.static_dir),
                "default-avatar.svg".to_string(),
            )),
        ),
    }
}

/// Get a circle's banner image
pub async fn banner_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let circle = match database.get_circle(id).await {
        Ok(ua) => ua,
        Err(_) => {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    pathd!("{}/images", database.config.static_dir),
                    "default-banner.svg".to_string(),
                )),
            );
        }
    };

    // ...
    let banner_url = match circle.metadata.kv.get("sparkler:banner_url") {
        Some(r) => r,
        None => "",
    };

    if banner_url.starts_with(&database.config.host) {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                pathd!("{}/images", database.config.static_dir),
                "default-banner.svg".to_string(),
            )),
        );
    }

    for host in database.config.blocked_hosts {
        if banner_url.starts_with(&host) {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    pathd!("{}/images", database.config.static_dir),
                    "default-banner.svg".to_string(),
                )),
            );
        }
    }

    // get profile image
    if banner_url.is_empty() {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                pathd!("{}/images", database.config.static_dir),
                "default-banner.svg".to_string(),
            )),
        );
    }

    let guessed_mime = mime_guess::from_path(banner_url)
        .first_raw()
        .unwrap_or("application/octet-stream");

    match database.auth.http.get(banner_url).send().await {
        Ok(stream) => {
            if let Some(ct) = stream.headers().get("Content-Type") {
                if !ct.to_str().unwrap().starts_with("image/") {
                    // if we failed to load the image, we might get back text/html or something
                    // we're going to return the default image if we got something that isn't
                    // an image (or has an incorrect mime)
                    return (
                        [("Content-Type", "image/svg+xml")],
                        Body::from(read_image(
                            pathd!("{}/images", database.config.static_dir),
                            "default-banner.svg".to_string(),
                        )),
                    );
                }
            }

            (
                [(
                    "Content-Type",
                    if guessed_mime == "text/html" {
                        "text/plain"
                    } else {
                        guessed_mime
                    },
                )],
                Body::from_stream(stream.bytes_stream()),
            )
        }
        Err(_) => (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                pathd!("{}/images", database.config.static_dir),
                "default-banner.svg".to_string(),
            )),
        ),
    }
}
