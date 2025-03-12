use crate::database::Database;
use crate::model::{DatabaseError, ProfileCreate, ProfileLogin, TokenContext};
use axum::http::{HeaderMap, HeaderValue};
use hcaptcha_no_wasm::Hcaptcha;
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};

/// [`Database::create_profile`]
pub async fn create_request(
    headers: HeaderMap,
    State(database): State<Database>,
    Json(props): Json<ProfileCreate>,
) -> impl IntoResponse {
    if !props.policy_consent {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DatabaseError::NotAllowed.to_json::<()>()).unwrap(),
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
    if database.get_ipban_by_ip(&real_ip).await.is_ok() {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DatabaseError::NotAllowed.to_json::<()>()).unwrap(),
        );
    }

    // create profile
    let res = match database.create_profile(props, &real_ip).await {
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
    let mut ua = match database.get_profile_by_username(&props.username).await {
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
    let input_password =
        rainbeam_shared::hash::hash_salted(props.password.clone(), ua.salt.clone());

    if input_password != ua.password {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DatabaseError::NotAllowed.to_json::<()>()).unwrap(),
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
    if database.get_ipban_by_ip(&real_ip).await.is_ok() {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DatabaseError::NotAllowed.to_json::<()>()).unwrap(),
        );
    }

    // check totp
    if !database.check_totp(&ua, &props.totp) {
        return (
            HeaderMap::new(),
            serde_json::to_string(&DatabaseError::NotAllowed.to_json::<()>()).unwrap(),
        );
    }

    // ...
    let token = databeam::utility::uuid();
    let token_hashed = databeam::utility::hash(token.clone());

    ua.tokens.push(token_hashed);
    ua.ips.push(real_ip);
    ua.token_context.push(TokenContext::default());

    database
        .update_profile_tokens(&props.username, ua.tokens, ua.ips, ua.token_context)
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

#[derive(serde::Deserialize)]
pub struct CallbackQueryProps {
    pub token: String, // this uid will need to be sent to the client as a token
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
                    params.token,
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

#[derive(Serialize, Deserialize)]
pub struct SetTokenQuery {
    #[serde(default)]
    pub token: String,
}

/// Set the current session token
pub async fn set_token_request(Query(props): Query<SetTokenQuery>) -> impl IntoResponse {
    (
        {
            let mut headers = HeaderMap::new();

            headers.insert(
                "Set-Cookie",
                format!(
                    "__Secure-Token={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
                    props.token,
                    60* 60 * 24 * 365
                )
                .parse()
                .unwrap(),
            );

            headers
        },
        "Token changed",
    )
}
