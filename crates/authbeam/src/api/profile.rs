use crate::database::Database;
use crate::model::{
    DatabaseError, NotificationCreate, Permission, SetProfileBadges, SetProfileCoins,
    SetProfileGroup, SetProfileLabels, SetProfileMetadata, SetProfilePassword, SetProfileTier,
    SetProfileUsername, TokenContext, TokenPermission,
};
use databeam::DefaultReturn;
use pathbufd::pathd;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

use std::{fs::File, io::Read};

pub fn read_image(static_dir: String, image: String) -> Vec<u8> {
    let mut bytes = Vec::new();

    for byte in File::open(format!("{static_dir}/{image}")).unwrap().bytes() {
        bytes.push(byte.unwrap())
    }

    bytes
}

/// Get a profile's avatar image
pub async fn avatar_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let auth_user = match database.get_profile(id).await {
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
    let avatar_url = match auth_user.metadata.kv.get("sparkler:avatar_url") {
        Some(r) => r,
        None => "",
    };

    if (avatar_url == "rb://") && !database.config.media_dir.to_string().is_empty() {
        return (
            [("Content-Type", "image/avif")],
            Body::from(read_image(
                pathd!("{}/avatars", database.config.media_dir),
                format!("{}.avif", auth_user.id.clone()),
            )),
        );
    }

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

    match database.http.get(avatar_url).send().await {
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

/// Get a profile's banner image
pub async fn banner_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let auth_user = match database.get_profile(id).await {
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
    let banner_url = match auth_user.metadata.kv.get("sparkler:banner_url") {
        Some(r) => r,
        None => "",
    };

    if (banner_url == "rb://") && !database.config.media_dir.to_string().is_empty() {
        return (
            [("Content-Type", "image/avif")],
            Body::from(read_image(
                pathd!("{}/banners", database.config.media_dir),
                format!("{}.avif", auth_user.id.clone()),
            )),
        );
    }

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

    match database.http.get(banner_url).send().await {
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

/// View a profile's information
pub async fn get_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user
    let mut auth_user = match database.get_profile(id).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            });
        }
    };

    // clean profile
    auth_user.clean();

    // return
    Json(DefaultReturn {
        success: true,
        message: auth_user.username.to_string(),
        payload: Some(auth_user),
    })
}

/// Change a profile's tier
pub async fn update_tier_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileTier>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::Moderator)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: None,
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // get other user
    let other_user = match database.get_profile(id.clone()).await {
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database.update_profile_tier(id, props.tier).await {
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

/// Change a profile's group
pub async fn update_group_request(
    jar: CookieJar,
    Path(id): Path<String>,
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
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // get other user
    let other_user = match database.get_profile(id.clone()).await {
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database
        .update_profile_group(other_user.id.clone(), props.group)
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    if let Err(e) = database
        .audit(
            auth_user.id,
            format!(
                "Changed user group: [{}](/+u/{})",
                other_user.id, other_user.id
            ),
        )
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    };

    Json(DefaultReturn {
        success: true,
        message: "Acceptable".to_string(),
        payload: Some(props.group),
    })
}

/// Change a profile's coins
pub async fn update_coins_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileCoins>,
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
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // get other user
    let other_user = match database.get_profile(id.clone()).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            });
        }
    };

    // push update
    // TODO: try not to clone
    if let Err(e) = database
        .update_profile_coins(other_user.id.clone(), props.coins)
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    if let Err(e) = database
        .audit(
            auth_user.id,
            format!(
                "Updated user coin balance: [{}](/+u/{})",
                other_user.id, other_user.id
            ),
        )
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    };

    Json(DefaultReturn {
        success: true,
        message: "Acceptable".to_string(),
        payload: Some(props.coins),
    })
}

/// Update the given user's session tokens
pub async fn update_tokens_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(req): Json<super::me::UpdateTokens>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::Moderator)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: (),
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: (),
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    let mut other = match database.get_profile(id).await {
        Ok(o) => o,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: (),
            });
        }
    };

    if auth_user.id == other.id {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // check permission
    let group = match database.get_group_by_id(other.group).await {
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // for every token that doesn't have a context, insert the default context
    for (i, _) in other.tokens.clone().iter().enumerate() {
        if let None = other.token_context.get(i) {
            other.token_context.insert(i, TokenContext::default())
        }
    }

    // get diff
    let mut removed_indexes = Vec::new();

    for (i, token) in other.tokens.iter().enumerate() {
        if !req.tokens.contains(token) {
            removed_indexes.push(i);
        }
    }

    // edit dependent vecs
    for i in removed_indexes.clone() {
        if (other.ips.len() < i) | (other.ips.len() == 0) {
            break;
        }

        other.ips.remove(i);
    }

    for i in removed_indexes.clone() {
        if (other.token_context.len() < i) | (other.token_context.len() == 0) {
            break;
        }

        other.token_context.remove(i);
    }

    // return
    if let Err(e) = database
        .update_profile_tokens(other.id, req.tokens, other.ips, other.token_context)
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

/// Generate a new token and session (like logging in while already logged in)
pub async fn generate_token_request(
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<TokenContext>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::Moderator)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: None,
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    let mut other = match database.get_profile(id).await {
        Ok(o) => o,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            });
        }
    };

    if auth_user.id == other.id {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // check permission
    let group = match database.get_group_by_id(other.group).await {
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // for every token that doesn't have a context, insert the default context
    for (i, _) in other.tokens.clone().iter().enumerate() {
        if let None = other.token_context.get(i) {
            other.token_context.insert(i, TokenContext::default())
        }
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
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // ...
    let token = databeam::utility::uuid();
    let token_hashed = databeam::utility::hash(token.clone());

    other.tokens.push(token_hashed);
    other.ips.push(String::new()); // don't actually store ip, this endpoint is used by external apps
    other.token_context.push(props);

    database
        .update_profile_tokens(other.id, other.tokens, other.ips, other.token_context)
        .await
        .unwrap();

    // return
    return Json(DefaultReturn {
        success: true,
        message: "Generated token!".to_string(),
        payload: Some(token),
    });
}

/// Change a profile's password
pub async fn update_password_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfilePassword>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::ManageAccount)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: None,
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    let mut is_manager = false;
    if auth_user.id != id && auth_user.username != id {
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
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
            });
        } else {
            is_manager = true;
        }

        // get other user
        let other_user = match database.get_profile(id.clone()).await {
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
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database
        .update_profile_password(id, props.password, props.new_password.clone(), !is_manager)
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
pub async fn update_username_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileUsername>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::ManageAccount)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: None,
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
            });
        }
    };

    // check permission
    if auth_user.id != id && auth_user.username != id {
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
                message: DatabaseError::NotAllowed.to_string(),
                payload: None,
            });
        }

        // get other user
        let other_user = match database.get_profile(id.clone()).await {
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
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // push update
    // TODO: try not to clone
    if let Err(e) = database
        .update_profile_username(id, props.password, props.new_name.clone())
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
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileMetadata>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::ManageProfile)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: (),
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: (),
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // check permission
    if auth_user.id != id && auth_user.username != id {
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
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }

        // get other user
        let other_user = match database.get_profile(id.clone()).await {
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
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    match database.update_profile_metadata(id, props.metadata).await {
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

/// Patch a user's metadata
pub async fn patch_metdata_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileMetadata>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::ManageProfile)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: (),
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: (),
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // get other user
    let other_user = match database.get_profile(id.clone()).await {
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
    if auth_user.id != id && auth_user.username != id {
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
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
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
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // patch metadata
    let mut metadata = other_user.metadata.clone();

    for kv in props.metadata.kv {
        metadata.kv.insert(kv.0, kv.1);
    }

    // return
    match database.update_profile_metadata(id, metadata).await {
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

/// Update a user's badges
pub async fn update_badges_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileBadges>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::Moderator)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: (),
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: (),
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // get other user
    let other_user = match database.get_profile(id.clone()).await {
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    match database.update_profile_badges(id, props.badges).await {
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

/// Update a user's labels
pub async fn update_labels_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetProfileLabels>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::Moderator)
                    {
                        return Json(DefaultReturn {
                            success: false,
                            message: DatabaseError::NotAllowed.to_string(),
                            payload: (),
                        });
                    }

                    // return
                    ua
                }
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: (),
                    });
                }
            }
        }
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // get other user
    let other_user = match database.get_profile(id.clone()).await {
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
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    match database.update_profile_labels(id, props.labels).await {
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
pub async fn delete_request(
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
                message: DatabaseError::NotAllowed.to_string(),
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
                message: DatabaseError::NotAllowed.to_string(),
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
                message: DatabaseError::NotAllowed.to_string(),
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
            message: DatabaseError::NotAllowed.to_string(),
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
