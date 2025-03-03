use crate::database::Database;
use crate::model::DatabaseError;
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

/// Delete a notification
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
    if let Err(e) = database.delete_notification(id, auth_user).await {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Notification deleted".to_string(),
        payload: (),
    })
}

/// Delete the current user's notifications
pub async fn delete_all_request(
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
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database
        .delete_notifications_by_recipient(auth_user.id.clone(), auth_user)
        .await
    {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Notifications cleared!".to_string(),
        payload: (),
    })
}
