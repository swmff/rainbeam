use crate::database::Database;
use crate::model::{DataExportOptions, DatabaseError, RelationshipStatus};
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue};
use axum_extra::extract::CookieJar;
use hcaptcha::Hcaptcha;
use std::{fs::File, io::Read};

use authbeam::model::{NotificationCreate, Permission, UserFollow};
use databeam::DefaultReturn;

use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
    Json, Router,
};

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/:username/avatar", get(avatar_request))
        .route("/:username/banner", get(banner_request))
        .route("/:username/report", post(report_request))
        .route("/:username/follow", post(follow_request))
        .route("/:username/export", get(export_request)) // staff
        .route("/:username/relationship/friend", post(friend_request))
        .route("/:username/relationship/block", post(block_request))
        .route("/:username/relationship", delete(breakup_request))
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

/// Redirect an ID to a full username
pub async fn expand_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    match database.get_profile(id).await {
        Ok(r) => Redirect::to(&format!("/@{}", r.username)),
        Err(_) => Redirect::to("/"),
    }
}

/// Redirect an IP to a full username
pub async fn expand_ip_request(
    jar: CookieJar,
    Path(ip): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    match jar.get("__Secure-Token") {
        Some(c) => {
            if let Err(_) = database
                .auth
                .get_profile_by_unhashed(c.value_trimmed().to_string())
                .await
            {
                return Redirect::to("/");
            }
        }
        None => {
            return Redirect::to("/");
        }
    };

    // return
    match database.auth.get_profile_by_ip(ip).await {
        Ok(r) => Redirect::to(&format!("/@{}", r.username)),
        Err(_) => Redirect::to("/"),
    }
}

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

    for host in database.server_options.blocked_hosts {
        if avatar_url.starts_with(&host) {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    database.server_options.static_dir,
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
                database.server_options.static_dir,
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
                            database.server_options.static_dir,
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

    for host in database.server_options.blocked_hosts {
        if banner_url.starts_with(&host) {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    database.server_options.static_dir,
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
                database.server_options.static_dir,
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
                            database.server_options.static_dir,
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
                database.server_options.static_dir,
                "default-banner.svg".to_string(),
            )),
        ),
    }
}

/// Report a user profile
pub async fn report_request(
    headers: HeaderMap,
    Path(input): Path<String>,
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
    let profile = match database.get_profile(input.clone()).await {
        Ok(p) => p,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
        }
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
        .create_notification(NotificationCreate {
            title: format!("**PROFILE REPORT**: [/@{input}](/+u/{})", profile.id),
            content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
            address: format!("/@{input}"),
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

    let relationship = database
        .auth
        .get_user_relationship(attempting_to_follow.id.clone(), auth_user.id.clone())
        .await
        .0;

    if relationship == RelationshipStatus::Blocked {
        // blocked users cannot follow the people who blocked them!
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
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

/// Create a data export of the given user
pub async fn export_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Query(props): Query<DataExportOptions>,
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
                    payload: None,
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::Other.to_string(),
                payload: None,
            })
        }
    };

    if !group.permissions.contains(&Permission::Helper) {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

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
                payload: None,
            })
        }
    };

    // return
    match database.create_data_export(other_user.id, props).await {
        Ok(export) => {
            return Json(DefaultReturn {
                success: true,
                message: "Acceptable".to_string(),
                payload: Some(export),
            })
        }
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    }
}

/// Send/accept a friend request to/from another user
pub async fn friend_request(
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
                    payload: None,
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
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
                payload: None,
            })
        }
    };

    // get current relationship
    let current = database
        .auth
        .get_user_relationship(auth_user.id.clone(), other_user.id.clone())
        .await;

    if current.0 == RelationshipStatus::Blocked && auth_user.id != current.1 {
        // cannot change relationship if we're blocked and we aren't the user that did the blocking
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    let current = current.0;

    // return
    if current == RelationshipStatus::Unknown {
        // send request
        match database
            .auth
            .set_user_relationship_status(
                auth_user.id,
                other_user.id,
                RelationshipStatus::Pending,
                false,
            )
            .await
        {
            Ok(export) => {
                return Json(DefaultReturn {
                    success: true,
                    message: "Friend request sent!".to_string(),
                    payload: Some(export),
                })
            }
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        }
    } else if current == RelationshipStatus::Pending {
        // accept request
        match database
            .auth
            .set_user_relationship_status(
                auth_user.id,
                other_user.id,
                RelationshipStatus::Friends,
                false,
            )
            .await
        {
            Ok(export) => {
                return Json(DefaultReturn {
                    success: true,
                    message: "Friend request accepted!".to_string(),
                    payload: Some(export),
                })
            }
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        }
    } else {
        // no clue, remove friendship?
        match database
            .auth
            .set_user_relationship_status(
                auth_user.id,
                other_user.id,
                RelationshipStatus::Unknown,
                false,
            )
            .await
        {
            Ok(export) => {
                return Json(DefaultReturn {
                    success: true,
                    message: "Friendship removed".to_string(),
                    payload: Some(export),
                })
            }
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        }
    }
}

/// Block another user
pub async fn block_request(
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
                    payload: None,
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
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
                payload: None,
            })
        }
    };

    // get current relationship
    let current = database
        .auth
        .get_user_relationship(auth_user.id.clone(), other_user.id.clone())
        .await;

    if current.0 == RelationshipStatus::Blocked && auth_user.id != current.1 {
        // cannot change relationship if we're blocked and we aren't the user that did the blocking
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // force unfollow
    if let Err(e) = database
        .auth
        .force_remove_user_follow(&mut UserFollow {
            user: auth_user.id.clone(),
            following: other_user.id.clone(),
        })
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    if let Err(e) = database
        .auth
        .force_remove_user_follow(&mut UserFollow {
            user: other_user.id.clone(),
            following: auth_user.id.clone(),
        })
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    match database
        .auth
        .set_user_relationship_status(
            auth_user.id,
            other_user.id,
            RelationshipStatus::Blocked,
            false,
        )
        .await
    {
        Ok(export) => {
            return Json(DefaultReturn {
                success: true,
                message: "User blocked!".to_string(),
                payload: Some(export),
            })
        }
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    }
}

/// Remove relationship with another user
pub async fn breakup_request(
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
                    payload: None,
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
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
                payload: None,
            })
        }
    };

    // get current relationship
    let current = database
        .auth
        .get_user_relationship(auth_user.id.clone(), other_user.id.clone())
        .await;

    if current.0 == RelationshipStatus::Blocked && auth_user.id != current.1 {
        // cannot remove relationship if we're blocked and we aren't the user that did the blocking
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // return
    match database
        .auth
        .set_user_relationship_status(
            auth_user.id,
            other_user.id,
            RelationshipStatus::Unknown,
            false,
        )
        .await
    {
        Ok(export) => {
            return Json(DefaultReturn {
                success: true,
                message: "Relationship removed!".to_string(),
                payload: Some(export),
            })
        }
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    }
}
