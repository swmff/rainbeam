use crate::database::Database;
use crate::model::{DatabaseError, TokenContext, TokenPermission};
use serde::{Deserialize, Serialize};
use databeam::DefaultReturn;

use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use axum_extra::extract::cookie::CookieJar;

/// Returns the current user's username
pub async fn get_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
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

    // return
    Json(DefaultReturn {
        success: true,
        message: auth_user.id.clone(),
        payload: Some(auth_user),
    })
}

#[derive(Serialize, Deserialize)]
pub struct DeleteProfile {
    password: String,
}

/// Delete the current user's profile
pub async fn delete_request(
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
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // get profile
    let hashed = rainbeam_shared::hash::hash_salted(req.password, auth_user.salt);

    if hashed != auth_user.password {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
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

/// Generate a new token and session (like logging in while already logged in)
pub async fn generate_token_request(
    jar: CookieJar,
    headers: HeaderMap,
    State(database): State<Database>,
    Json(props): Json<TokenContext>,
) -> impl IntoResponse {
    // get user from token
    let mut existing_permissions: Option<Vec<TokenPermission>> = None;
    let mut auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed().to_string();

            match database.get_profile_by_unhashed(token.clone()).await {
                Ok(ua) => {
                    // check token permission
                    let token = ua.token_context_from_token(&token);

                    if let Some(ref permissions) = token.permissions {
                        existing_permissions = Some(permissions.to_owned())
                    }

                    if !token.can_do(TokenPermission::GenerateTokens) {
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

    // for every token that doesn't have a context, insert the default context
    for (i, _) in auth_user.tokens.clone().iter().enumerate() {
        if let None = auth_user.token_context.get(i) {
            auth_user.token_context.insert(i, TokenContext::default())
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

    // check given context
    if let Some(ref permissions) = props.permissions {
        // make sure we don't want anything we don't have
        // if our permissions are "None", allow any permission to be granted
        for permission in permissions {
            if let Some(ref existing) = existing_permissions {
                if !existing.contains(permission) {
                    return Json(DefaultReturn {
                        success: false,
                        message: DatabaseError::OutOfScope.to_string(),
                        payload: None,
                    });
                }
            } else {
                break;
            }
        }
    }

    // ...
    let token = databeam::utility::uuid();
    let token_hashed = databeam::utility::hash(token.clone());

    auth_user.tokens.push(token_hashed);
    auth_user.ips.push(String::new()); // don't actually store ip, this endpoint is used by external apps
    auth_user.token_context.push(props);

    database
        .update_profile_tokens(
            auth_user.id,
            auth_user.tokens,
            auth_user.ips,
            auth_user.token_context,
        )
        .await
        .unwrap();

    // return
    return Json(DefaultReturn {
        success: true,
        message: "Generated token!".to_string(),
        payload: Some(token),
    });
}

#[derive(Serialize, Deserialize)]
pub struct UpdateTokens {
    pub tokens: Vec<String>,
}

/// Update the current user's session tokens
pub async fn update_tokens_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(req): Json<UpdateTokens>,
) -> impl IntoResponse {
    // get user from token
    let mut auth_user = match jar.get("__Secure-Token") {
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

    // for every token that doesn't have a context, insert the default context
    for (i, _) in auth_user.tokens.clone().iter().enumerate() {
        if let None = auth_user.token_context.get(i) {
            auth_user.token_context.insert(i, TokenContext::default())
        }
    }

    // get diff
    let mut removed_indexes = Vec::new();

    for (i, token) in auth_user.tokens.iter().enumerate() {
        if !req.tokens.contains(token) {
            removed_indexes.push(i);
        }
    }

    // edit dependent vecs
    for i in removed_indexes.clone() {
        if (auth_user.ips.len() < i) | (auth_user.ips.len() == 0) {
            break;
        }

        auth_user.ips.remove(i);
    }

    for i in removed_indexes.clone() {
        if (auth_user.token_context.len() < i) | (auth_user.token_context.len() == 0) {
            break;
        }

        auth_user.token_context.remove(i);
    }

    // return
    if let Err(e) = database
        .update_profile_tokens(
            auth_user.id,
            req.tokens,
            auth_user.ips,
            auth_user.token_context,
        )
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
