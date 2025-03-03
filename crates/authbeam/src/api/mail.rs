use crate::database::Database;
use crate::model::{DatabaseError, MailCreate, SetMailState, TokenPermission};
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

/// Create a mail
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(props): Json<MailCreate>,
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
                        .can_do(TokenPermission::SendMail)
                    {
                        return Json(DatabaseError::NotAllowed.to_json());
                    }

                    // return
                    ua
                }
                Err(e) => return Json(e.to_json()),
            }
        }
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    let mail = match database.create_mail(props, auth_user.id.clone()).await {
        Ok(m) => m,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    Json(DefaultReturn {
        success: true,
        message: "Mail created".to_string(),
        payload: Some(mail),
    })
}

/// Delete a mail
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
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database.delete_mail(id, auth_user).await {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Mail deleted".to_string(),
        payload: (),
    })
}

/// Update mail state
pub async fn update_state_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetMailState>,
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
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database.update_mail_state(id, props.state, auth_user).await {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Mail updated".to_string(),
        payload: (),
    })
}
