use crate::database::Database;
use crate::model::{DataExportOptions, DatabaseError};
use crate::ToHtml;
use axum::body::Body;
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue, Response};
use axum_extra::extract::CookieJar;
use hcaptcha_no_wasm::Hcaptcha;

use authbeam::model::{FinePermission, IpBlockCreate, NotificationCreate};
use databeam::prelude::DefaultReturn;

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/{id}/report", post(report_request))
        .route("/{id}/export", get(export_request)) // staff
        .route("/{id}/ipblock", post(ipblock_request))
        // ...
        .with_state(database)
}

// routes

/// Redirect an ID to a full username
pub async fn expand_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> Response<Body> {
    match database.get_profile(id).await {
        Ok(r) => Redirect::to(&format!("/@{}", r.username)).into_response(),
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/html")],
            DatabaseError::NotFound.to_html(database),
        )
            .into_response(),
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
                .get_profile_by_unhashed(c.value_trimmed())
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
    match database.auth.get_profile_by_ip(&ip).await {
        Ok(r) => Redirect::to(&format!("/@{}", r.username)),
        Err(_) => Redirect::to("/"),
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
        .valid_response(&database.config.captcha.secret, None)
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
        Err(e) => return Json(e.to_json()),
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
                title: format!("**PROFILE REPORT**: [/@{input}](/+u/{})", profile.id),
                content: format!("{}\n\n***\n\n[{real_ip}](/+i/{real_ip})", req.content),
                address: format!("/@{input}"),
                recipient: "*".to_string(), // all staff
            },
            None,
        )
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
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
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

    if !group.permissions.check(FinePermission::EXPORT_DATA) {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // ...
    let other_user = match database.auth.get_profile_by_username(&username).await {
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
        Err(e) => return Json(e.to_json()),
    }
}

/// IP block a profile
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

    // get profile
    let profile = match database.auth.get_profile(&id).await {
        Ok(p) => p,
        Err(e) => return Json(e.to_json()),
    };

    // block
    for ip in profile.ips {
        if let Err(_) = database
            .auth
            .create_ipblock(
                IpBlockCreate {
                    ip,
                    context: profile.username.clone(),
                },
                auth_user.clone(),
            )
            .await
        {
            continue;
        }
    }

    return Json(DefaultReturn {
        success: true,
        message: "IPs blocked!".to_string(),
        payload: (),
    });
}
