//! Responds to API requests
use crate::database::Database;
use crate::model::{
    AuthError, IpBanCreate, IpBlockCreate, NotificationCreate, Permission, ProfileCreate,
    ProfileLogin, SetProfileBadges, SetProfileGroup, SetProfileMetadata, SetProfilePassword,
    SetProfileTier, SetProfileUsername, WarningCreate,
};
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderValue};
use hcaptcha::Hcaptcha;
use serde::{Deserialize, Serialize};
use databeam::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::cookie::CookieJar;

pub fn routes(database: Database) -> Router {
    Router::new()
        // profiles
        // .route("/profile/:username/group", post(set_group_request))
        .route("/profile/:username/tier", post(set_tier_request))
        .route("/profile/:username/password", post(set_password_request))
        .route("/profile/:username/username", post(set_username_request))
        .route("/profile/:username/metadata", post(update_metdata_request))
        .route("/profile/:username/badges", post(update_badges_request))
        .route("/profile/:username/avatar", get(profile_avatar_request))
        .route("/profile/:username", delete(delete_other_request))
        .route("/profile/:username", get(profile_inspect_request))
        // notifications
        .route("/notifications/:id", delete(delete_notification_request))
        .route(
            "/notifications/clear",
            delete(delete_all_notifications_request),
        )
        // warnings
        .route("/warnings", post(create_warning_request))
        .route("/warnings/:id", delete(delete_warning_request))
        // ipbans
        .route("/ipbans", post(create_ipban_request))
        .route("/ipbans/:id", delete(delete_ipban_request))
        // ipblocks
        .route("/ipblocks", post(create_ipblock_request))
        .route("/ipblocks/:id", delete(delete_ipblock_request))
        // me
        .route("/me/tokens", post(update_my_tokens_request))
        .route("/me/delete", post(delete_me_request))
        .route("/me", get(me_request))
        // account
        .route("/register", post(create_profile_request))
        .route("/login", post(login_request))
        .route("/callback", get(callback_request))
        .route("/logout", post(logout_request))
        .route("/untag", post(remove_tag))
        // ...
        .with_state(database)
}

/// [`Database::create_profile`]
pub async fn create_profile_request(
    jar: CookieJar,
    headers: HeaderMap,
    State(database): State<Database>,
    Json(props): Json<ProfileCreate>,
) -> impl IntoResponse {
    if let Some(_) = jar.get("__Secure-Token") {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            })
            .unwrap(),
        );
    }

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
    if database.get_ipban_by_ip(real_ip.clone()).await.is_ok() {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            })
            .unwrap(),
        );
    }

    // create profile
    let res = match database.create_profile(props, real_ip).await {
        Ok(r) => r,
        Err(e) => {
            return (
                HeaderMap::new(),
                serde_json::to_string(&DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                })
                .unwrap(),
            );
        }
    };

    // return
    let mut headers = HeaderMap::new();

    headers.insert(
        "Set-Cookie",
        format!(
            "__Secure-Token={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
            res,
            60* 60 * 24 * 365
        )
        .parse()
        .unwrap(),
    );

    (
        headers,
        serde_json::to_string(&DefaultReturn {
            success: true,
            message: res.clone(),
            payload: (),
        })
        .unwrap(),
    )
}

/// [`Database::get_profile_by_username_password`]
pub async fn login_request(
    headers: HeaderMap,
    State(database): State<Database>,
    Json(props): Json<ProfileLogin>,
) -> impl IntoResponse {
    // check hcaptcha
    if let Err(e) = props
        .valid_response(&database.config.captcha.secret, None)
        .await
    {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
            .unwrap(),
        );
    }

    // ...
    let mut ua = match database
        .get_profile_by_username(props.username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return (
                HeaderMap::new(),
                serde_json::to_string(&DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                })
                .unwrap(),
            )
        }
    };

    // check password
    let input_password = shared::hash::hash_salted(props.password.clone(), ua.salt);

    if input_password != ua.password {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            })
            .unwrap(),
        );
    }

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
    if database.get_ipban_by_ip(real_ip.clone()).await.is_ok() {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            })
            .unwrap(),
        );
    }

    // ...
    let token = databeam::utility::uuid();
    let token_hashed = databeam::utility::hash(token.clone());

    ua.tokens.push(token_hashed);
    ua.ips.push(real_ip);

    database
        .edit_profile_tokens_by_name(props.username.clone(), ua.tokens, ua.ips)
        .await
        .unwrap();

    // return
    let mut headers = HeaderMap::new();

    headers.insert(
        "Set-Cookie",
        format!(
            "__Secure-Token={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
            token,
            60* 60 * 24 * 365
        )
        .parse()
        .unwrap(),
    );

    (
        headers,
        serde_json::to_string(&DefaultReturn {
            success: true,
            message: token,
            payload: (),
        })
        .unwrap(),
    )
}

/// Delete a notification
pub async fn delete_notification_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    if let Err(e) = database.delete_notification(id, auth_user).await {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: (),
        });
    }

    Json(DefaultReturn {
        success: true,
        message: "Notification deleted".to_string(),
        payload: (),
    })
}

/// Delete the current user's notifications
pub async fn delete_all_notifications_request(
    jar: CookieJar,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    if let Err(e) = database
        .delete_notifications_by_recipient(auth_user.id.clone(), auth_user)
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: (),
        });
    }

    Json(DefaultReturn {
        success: true,
        message: "Notifications cleared!".to_string(),
        payload: (),
    })
}

/// Returns the current user's username
pub async fn me_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    Json(DefaultReturn {
        success: true,
        message: auth_user.username,
        payload: (),
    })
}

#[derive(Serialize, Deserialize)]
pub struct DeleteProfile {
    password: String,
}

/// Delete the current user's profile
pub async fn delete_me_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(req): Json<DeleteProfile>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // get profile
    let hashed = shared::hash::hash_salted(req.password, auth_user.salt);

    if hashed != auth_user.password {
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    if let Err(e) = database.delete_profile_by_id(auth_user.id).await {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: (),
        });
    }

    Json(DefaultReturn {
        success: true,
        message: "Profile deleted, goodbye!".to_string(),
        payload: (),
    })
}

#[derive(Serialize, Deserialize)]
pub struct UpdateTokens {
    tokens: Vec<String>,
}

/// Update the current user's session tokens
pub async fn update_my_tokens_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(req): Json<UpdateTokens>,
) -> impl IntoResponse {
    // get user from token
    let mut auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // edit ips
    let mut removed_indexes = Vec::new();

    for (i, token) in auth_user.tokens.iter().enumerate() {
        if !req.tokens.contains(token) {
            removed_indexes.push(i);
        }
    }

    for i in removed_indexes {
        if (auth_user.ips.len() < i) | (auth_user.ips.len() == 0) {
            break;
        }

        auth_user.ips.remove(i);
    }

    // return
    if let Err(e) = database
        .edit_profile_tokens_by_name(auth_user.username, req.tokens, auth_user.ips)
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: (),
        });
    }

    Json(DefaultReturn {
        success: true,
        message: "Tokens updated!".to_string(),
        payload: (),
    })
}

/// Get a profile's avatar image
pub async fn profile_avatar_request(
    Path(username): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let auth_user = match database.get_profile_by_username(username).await {
        Ok(ua) => ua,
        Err(_) => {
            return Bytes::from_static(&[0x0u8]);
        }
    };

    // get profile image
    if auth_user.metadata.avatar_url.is_empty() {
        return Bytes::from_static(&[0]);
    }

    match database
        .http
        .get(auth_user.metadata.avatar_url)
        .send()
        .await
    {
        Ok(r) => r.bytes().await.unwrap(),
        Err(_) => Bytes::from_static(&[0x0u8]),
    }
}

/// View a profile's information
pub async fn profile_inspect_request(
    Path(username): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let mut auth_user = match database.get_profile_by_username(username).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            });
        }
    };

    // edit profile
    auth_user.salt = String::new();
    auth_user.password = String::new();
    auth_user.tokens = Vec::new();
    auth_user.ips = Vec::new();

    // return
    Json(DefaultReturn {
        success: true,
        message: auth_user.username.to_string(),
        payload: Some(auth_user),
    })
}

/// Change a profile's group
pub async fn set_group_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileGroup>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    let group = match database.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    if !group.permissions.contains(&Permission::Manager) {
        // we must have the "Manager" permission to edit other users
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // get other user
    let other_user = match database.get_profile_by_username(username.clone()).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    let group = match database.get_group_by_id(other_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    if group.permissions.contains(&Permission::Manager) {
        // we cannot manager other managers
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database.edit_profile_group(username, props.group).await {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    Json(DefaultReturn {
        success: true,
        message: "Acceptable".to_string(),
        payload: Some(props.group),
    })
}

/// Change a profile's tier
pub async fn set_tier_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileTier>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    let group = match database.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    if !group.permissions.contains(&Permission::Manager) {
        // we must have the "Manager" permission to edit other users
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // get other user
    let other_user = match database.get_profile_by_username(username.clone()).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    let group = match database.get_group_by_id(other_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    if group.permissions.contains(&Permission::Manager) {
        // we cannot manager other managers
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database.edit_profile_tier(username, props.tier).await {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    Json(DefaultReturn {
        success: true,
        message: "Acceptable".to_string(),
        payload: Some(props.tier),
    })
}

/// Change a profile's password
pub async fn set_password_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfilePassword>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    let mut is_manager = false;
    if auth_user.username != username {
        let group = match database.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        };

        if !group.permissions.contains(&Permission::Manager) {
            // we must have the "Manager" permission to edit other users
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        } else {
            is_manager = true;
        }

        // get other user
        let other_user = match database.get_profile_by_username(username.clone()).await {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                });
            }
        };

        // check permission
        let group = match database.get_group_by_id(other_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        };

        if group.permissions.contains(&Permission::Manager) {
            // we cannot manager other managers
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        }
    }

    // check user permissions
    // returning NotAllowed here will block them from editing their profile
    // we don't want to waste resources on rule breakers
    if auth_user.group == -1 {
        // group -1 (even if it exists) is for marking users as banned
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database
        .edit_profile_password_by_name(
            username,
            props.password,
            props.new_password.clone(),
            is_manager == false,
        )
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    Json(DefaultReturn {
        success: true,
        message: "Acceptable".to_string(),
        payload: Some(props.new_password),
    })
}

/// Change a profile's username
pub async fn set_username_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileUsername>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    if auth_user.username != username {
        let group = match database.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        };

        if !group.permissions.contains(&Permission::Manager) {
            // we must have the "Manager" permission to edit other users
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        }

        // get other user
        let other_user = match database.get_profile_by_username(username.clone()).await {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                });
            }
        };

        // check permission
        let group = match database.get_group_by_id(other_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        };

        if group.permissions.contains(&Permission::Manager) {
            // we cannot manager other managers
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: None,
            });
        }
    }

    // check user permissions
    // returning NotAllowed here will block them from editing their profile
    // we don't want to waste resources on rule breakers
    if auth_user.group == -1 {
        // group -1 (even if it exists) is for marking users as banned
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database
        .edit_profile_username_by_name(username, props.password, props.new_name.clone())
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    Json(DefaultReturn {
        success: true,
        message: "Acceptable".to_string(),
        payload: Some(props.new_name),
    })
}

/// Update a user's metadata
pub async fn update_metdata_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileMetadata>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // check permission
    if auth_user.username != username {
        let group = match database.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                })
            }
        };

        if !group.permissions.contains(&Permission::Manager) {
            // we must have the "Manager" permission to edit other users
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }

        // get other user
        let other_user = match database.get_profile_by_username(username.clone()).await {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                });
            }
        };

        // check permission
        let group = match database.get_group_by_id(other_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                })
            }
        };

        if group.permissions.contains(&Permission::Manager) {
            // we cannot manager other managers
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    }

    // check user permissions
    // returning NotAllowed here will block them from editing their profile
    // we don't want to waste resources on rule breakers
    if auth_user.group == -1 {
        // group -1 (even if it exists) is for marking users as banned
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    match database
        .edit_profile_metadata_by_name(username, props.metadata)
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

/// Update a user's metadata
pub async fn update_badges_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileBadges>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // check permission
    let group = match database.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
        }
    };

    if !group.permissions.contains(&Permission::Helper) {
        // we must have the "Helper" permission to edit other users' badges
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // get other user
    let other_user = match database.get_profile_by_username(username.clone()).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            });
        }
    };

    // check permission
    let other_group = match database.get_group_by_id(other_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            })
        }
    };

    if other_group.permissions.contains(&Permission::Helper)
        && !group.permissions.contains(&Permission::Manager)
    {
        // we cannot manage other helpers without manager
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    match database
        .edit_profile_badges_by_name(username, props.badges)
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

/// Delete another user
pub async fn delete_other_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // check permission
    if auth_user.username != id {
        let group = match database.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                })
            }
        };

        // get other user
        let other_user = match database.get_profile_by_id(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                });
            }
        };

        if !group.permissions.contains(&Permission::Manager) {
            // we must have the "Manager" permission to edit other users
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        } else {
            let actor_id = auth_user.id;
            if let Err(e) = database
                .create_notification(
                    NotificationCreate {
                        title: format!("[{actor_id}](/+u/{actor_id})"),
                        content: format!("Deleted a profile: @{}", other_user.username),
                        address: format!("/+u/{actor_id}"),
                        recipient: "*(audit)".to_string(), // all staff, audit
                    },
                    None,
                )
                .await
            {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                });
            }
        }

        // check permission
        let group = match database.get_group_by_id(other_user.group).await {
            Ok(g) => g,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                })
            }
        };

        if group.permissions.contains(&Permission::Manager) {
            // we cannot manager other managers
            return Json(DefaultReturn {
                success: false,
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    }

    // check user permissions
    // returning NotAllowed here will block them from editing their profile
    // we don't want to waste resources on rule breakers
    if auth_user.group == -1 {
        // group -1 (even if it exists) is for marking users as banned
        return Json(DefaultReturn {
            success: false,
            message: AuthError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    match database.delete_profile_by_id(id).await {
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

/// Create a warning
pub async fn create_warning_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(props): Json<WarningCreate>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    match database.create_warning(props, auth_user).await {
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

/// Delete a warning
pub async fn delete_warning_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    match database.delete_warning(id, auth_user).await {
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

/// Create a ipban
pub async fn create_ipban_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(props): Json<IpBanCreate>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    match database.create_ipban(props, auth_user).await {
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

/// Delete an ipban
pub async fn delete_ipban_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    match database.delete_ipban(id, auth_user).await {
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

/// Create a ipblock
pub async fn create_ipblock_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(props): Json<IpBlockCreate>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    match database.create_ipblock(props, auth_user).await {
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

/// Delete an ipblock
pub async fn delete_ipblock_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
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
                message: AuthError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // return
    match database.delete_ipblock(id, auth_user).await {
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

// general
pub async fn not_found() -> impl IntoResponse {
    Json(DefaultReturn::<u16> {
        success: false,
        message: String::from("Path does not exist"),
        payload: 404,
    })
}

// auth
#[derive(serde::Deserialize)]
pub struct CallbackQueryProps {
    pub uid: String, // this uid will need to be sent to the client as a token
}

pub async fn callback_request(Query(params): Query<CallbackQueryProps>) -> impl IntoResponse {
    // return
    (
        [
            ("Content-Type".to_string(), "text/html".to_string()),
            (
                "Set-Cookie".to_string(),
                format!(
                    "__Secure-Token={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
                    params.uid,
                    60 * 60 * 24 * 365
                ),
            ),
        ],
        "<head>
            <meta http-equiv=\"Refresh\" content=\"0; URL=/\" />
        </head>"
    )
}

pub async fn logout_request(jar: CookieJar) -> impl IntoResponse {
    // check for cookie
    if let Some(_) = jar.get("__Secure-Token") {
        return (
            [
                ("Content-Type".to_string(), "text/plain".to_string()),
                (
                    "Set-Cookie".to_string(),
                    "__Secure-Token=refresh; SameSite=Strict; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age=0".to_string(),
                )            ],
            "You have been signed out. You can now close this tab.",
        );
    }

    // return
    (
        [
            ("Content-Type".to_string(), "text/plain".to_string()),
            ("Set-Cookie".to_string(), String::new()),
        ],
        "Failed to sign out of account.",
    )
}

pub async fn remove_tag(jar: CookieJar) -> impl IntoResponse {
    // check for cookie
    // anonymous users cannot remove their own tag
    if let Some(_) = jar.get("__Secure-Token") {
        return (
            [
                ("Content-Type".to_string(), "text/plain".to_string()),
                (
                    "Set-Cookie2".to_string(),
                    "__Secure-Question-Tag=refresh; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age=0".to_string()
                )
            ],
            "You have been signed out. You can now close this tab.",
        );
    }

    // return
    (
        [
            ("Content-Type".to_string(), "text/plain".to_string()),
            ("Set-Cookie".to_string(), String::new()),
        ],
        "Failed to remove tag.",
    )
}
