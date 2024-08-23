use crate::database::Database;
use crate::model::DatabaseError;
use axum_extra::extract::CookieJar;
use hcaptcha::Hcaptcha;
use std::{fs::File, io::Read};
use xsu_authman::model::{NotificationCreate, UserFollow};
use xsu_dataman::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    body::Body,
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/:username/avatar", get(avatar_request))
        .route("/:username/banner", get(banner_request))
        .route("/:username/report", post(report_request))
        .route("/:username/follow", post(follow_request))
        .route("/:username/unfollow", post(unfollow_request)) // force unfollow
        .route("/:username/unfollow-me", post(unfollow_me_request)) // force them to unfollow you
        // ...
        .with_state(database)
}

pub fn read_image(static_dir: String, image: String) -> Vec<u8> {
    let mut bytes = Vec::new();

    for byte in File::open(format!("{static_dir}/images/{image}",))
        .unwrap()
        .bytes()
    {
        bytes.push(byte.unwrap())
    }

    bytes
}

// routes

/// Get a profile's avatar image
pub async fn avatar_request(
    Path(username): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let auth_user = match database.auth.get_profile_by_username(username).await {
        Ok(ua) => ua,
        Err(_) => {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    database.server_options.static_dir,
                    "default-avatar.svg".to_string(),
                )),
            );
        }
    };

    // ...
    let avatar_url = match auth_user.metadata.kv.get("sparkler:avatar_url") {
        Some(r) => r,
        None => "",
    };

    if avatar_url.starts_with(&database.server_options.host) {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.server_options.static_dir,
                "default-avatar.svg".to_string(),
            )),
        );
    }

    // get profile image
    if avatar_url.is_empty() {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.server_options.static_dir,
                "default-avatar.svg".to_string(),
            )),
        );
    }

    let guessed_mime = mime_guess::from_path(avatar_url)
        .first_raw()
        .unwrap_or("application/octet-stream");

    match database.auth.http.get(avatar_url).send().await {
        Ok(stream) => (
            [(
                "Content-Type",
                if guessed_mime == "text/html" {
                    "text/plain"
                } else {
                    guessed_mime
                },
            )],
            Body::from_stream(stream.bytes_stream()),
        ),
        Err(_) => (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.server_options.static_dir,
                "default-avatar.svg".to_string(),
            )),
        ),
    }
}

/// Get a profile's banner image
pub async fn banner_request(
    Path(username): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let auth_user = match database.auth.get_profile_by_username(username).await {
        Ok(ua) => ua,
        Err(_) => {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    database.server_options.static_dir,
                    "default-banner.svg".to_string(),
                )),
            );
        }
    };

    // ...
    let banner_url = match auth_user.metadata.kv.get("sparkler:banner_url") {
        Some(r) => r,
        None => "",
    };

    if banner_url.starts_with(&database.server_options.host) {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.server_options.static_dir,
                "default-banner.svg".to_string(),
            )),
        );
    }

    // get profile image
    if banner_url.is_empty() {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.server_options.static_dir,
                "default-banner.svg".to_string(),
            )),
        );
    }

    let guessed_mime = mime_guess::from_path(banner_url)
        .first_raw()
        .unwrap_or("application/octet-stream");

    match database.auth.http.get(banner_url).send().await {
        Ok(stream) => (
            [(
                "Content-Type",
                if guessed_mime == "text/html" {
                    "text/plain"
                } else {
                    guessed_mime
                },
            )],
            Body::from_stream(stream.bytes_stream()),
        ),
        Err(_) => (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.server_options.static_dir,
                "default-banner.svg".to_string(),
            )),
        ),
    }
}

/// Report a user profile
pub async fn report_request(
    Path(username): Path<String>,
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

    // get user
    if let Err(_) = database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotFound.to_string(),
            payload: (),
        });
    };

    match database
        .auth
        .create_notification(NotificationCreate {
            title: format!("**PROFILE REPORT**: [/@{username}](/@{username})"),
            content: req.content,
            address: format!("/@{username}"),
            recipient: "*".to_string(), // all staff
        })
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Profile reported!".to_string(),
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

/// Toggle following on the given user
pub async fn follow_request(
    jar: CookieJar,
    Path(username): Path<String>,
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
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // check block status
    let attempting_to_follow = match database
        .auth
        .get_profile_by_username(username.to_owned())
        .await
    {
        Ok(ua) => ua,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotFound.to_string(),
                payload: (),
            })
        }
    };

    if attempting_to_follow
        .metadata
        .kv
        .get("sparkler:block_list")
        .unwrap_or(&String::new())
        .contains(&format!("<@{}>", auth_user.username))
    {
        // remove the user follow and return
        // blocked users cannot follow the people who blocked them!
        match database
            .auth
            .force_remove_user_follow(&mut UserFollow {
                user: auth_user.id,
                following: attempting_to_follow.id,
            })
            .await
        {
            Ok(_) => {
                return Json(DefaultReturn {
                    success: true,
                    message: "Acceptable".to_string(),
                    payload: (),
                })
            }
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                })
            }
        }
    }

    // return
    match database
        .auth
        .toggle_user_follow(&mut UserFollow {
            user: auth_user.id,
            following: attempting_to_follow.id,
        })
        .await
    {
        Ok(_) => Json(DefaultReturn {
            success: true,
            message: "Acceptable".to_string(),
            payload: (),
        }),
        Err(e) => Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: (),
        }),
    }
}

/// Force unfollow on the given user
pub async fn unfollow_request(
    jar: CookieJar,
    Path(username): Path<String>,
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
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // ...
    let attempting_to_follow = match database
        .auth
        .get_profile_by_username(username.to_owned())
        .await
    {
        Ok(ua) => ua,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotFound.to_string(),
                payload: (),
            })
        }
    };

    // return
    match database
        .auth
        .force_remove_user_follow(&mut UserFollow {
            user: auth_user.id,
            following: attempting_to_follow.id,
        })
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Acceptable".to_string(),
                payload: (),
            })
        }
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
        }
    }
}

/// Force unfollow the current user on the given user
pub async fn unfollow_me_request(
    jar: CookieJar,
    Path(username): Path<String>,
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
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // ...
    let other_user = match database
        .auth
        .get_profile_by_username(username.to_owned())
        .await
    {
        Ok(ua) => ua,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotFound.to_string(),
                payload: (),
            })
        }
    };

    // return
    match database
        .auth
        .force_remove_user_follow(&mut UserFollow {
            user: other_user.id,
            following: auth_user.id,
        })
        .await
    {
        Ok(_) => {
            return Json(DefaultReturn {
                success: true,
                message: "Acceptable".to_string(),
                payload: (),
            })
        }
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
        }
    }
}
