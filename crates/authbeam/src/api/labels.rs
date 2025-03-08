use crate::database::Database;
use crate::model::{DatabaseError, LabelCreate, TokenPermission};
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

/// Get a label
pub async fn get_request(
    State(database): State<Database>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    // get label
    let label = match database.get_label(id).await {
        Ok(i) => i,
        Err(e) => return Json(e.to_json()),
    };

    // return
    Json(DefaultReturn {
        success: true,
        message: id.to_string(),
        payload: Some(label),
    })
}

/// Create a label
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(props): Json<LabelCreate>,
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
                        .can_do(TokenPermission::ManageAssets)
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
    let label = match database
        .create_label(props.name, props.id, auth_user.id.clone())
        .await
    {
        Ok(m) => m,
        Err(e) => return Json(e.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: "Label created".to_string(),
        payload: Some(label),
    })
}

/// Delete a label
pub async fn delete_request(
    jar: CookieJar,
    Path(id): Path<i64>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database.delete_label(id, auth_user).await {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Label deleted".to_string(),
        payload: (),
    })
}
