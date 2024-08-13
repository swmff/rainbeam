use crate::database::Database;
use crate::model::DatabaseError;
use axum::body::Bytes;
use axum::routing::post;
use axum::Json;
use serde::{Deserialize, Serialize};
use xsu_authman::model::NotificationCreate;
use xsu_dataman::DefaultReturn;
use std::{io::Read, fs::File};

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/:username/avatar", get(avatar_request))
        .route("/:username/banner", get(banner_request))
        .route("/:username/report", post(report_request))
        // ...
        .with_state(database)
}

fn read_image(static_dir: String, image: String) -> Vec<u8> {
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
                Bytes::copy_from_slice(&read_image(
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

    // get profile image
    if avatar_url.is_empty() {
        return (
            [("Content-Type", "image/svg+xml")],
            Bytes::copy_from_slice(&read_image(
                database.server_options.static_dir,
                "default-avatar.svg".to_string(),
            )),
        );
    }

    match database.auth.http.get(avatar_url).send().await {
        Ok(r) => (
            [(
                "Content-Type",
                mime_guess::from_path(avatar_url)
                    .first_raw()
                    .unwrap_or("application/octet-stream"),
            )],
            r.bytes().await.unwrap(),
        ),
        Err(_) => (
            [("Content-Type", "image/svg+xml")],
            Bytes::copy_from_slice(&read_image(
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
                Bytes::copy_from_slice(&read_image(
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

    // get profile image
    if banner_url.is_empty() {
        return (
            [("Content-Type", "image/svg+xml")],
            Bytes::copy_from_slice(&read_image(
                database.server_options.static_dir,
                "default-banner.svg".to_string(),
            )),
        );
    }

    match database.auth.http.get(banner_url).send().await {
        Ok(r) => (
            [(
                "Content-Type",
                mime_guess::from_path(banner_url)
                    .first_raw()
                    .unwrap_or("application/octet-stream"),
            )],
            r.bytes().await.unwrap(),
        ),
        Err(_) => (
            [("Content-Type", "image/svg+xml")],
            Bytes::copy_from_slice(&read_image(
                database.server_options.static_dir,
                "default-banner.svg".to_string(),
            )),
        ),
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateReport {
    content: String,
}

/// Report a user profile
pub async fn report_request(
    Path(username): Path<String>,
    State(database): State<Database>,
    Json(req): Json<CreateReport>,
) -> impl IntoResponse {
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
