use axum::response::IntoResponse;
use axum::Json;
use axum::extract::{Query, State, Path};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use authbeam::model::{Permission, Profile, RelationshipStatus};

use databeam::DefaultReturn;
use crate::database::Database;
use crate::model::{Chat, DatabaseError, Message};
use super::PaginatedQuery;

#[derive(Serialize, Deserialize)]
struct HomepageTemplate {
    chats: Vec<(Chat, Vec<Box<Profile>>)>,
}

/// GET /chats
pub async fn chats_homepage_request(
    jar: CookieJar,
    State(database): State<Database>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let chats = match database.get_chats_for_user(auth_user.id.clone()).await {
        Ok(c) => c,
        Err(e) => return Json(e.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(HomepageTemplate { chats }),
    })
}

#[derive(Serialize, Deserialize)]
struct ChatTemplate {
    chat: Chat,
    members: Vec<Box<Profile>>,
    messages: Vec<(Message, Box<Profile>)>,
    friends: Vec<(Box<Profile>, Box<Profile>)>,
    last_message_id: String,
    is_helper: bool,
    page: i32,
}

/// GET /chats/{id}
pub async fn chat_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Query(props): Query<PaginatedQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let chat = match database.get_chat(id.clone()).await {
        Ok(c) => c,
        Err(e) => return Json(e.to_json()),
    };

    let messages = match database
        .get_messages_by_chat_paginated(id.clone(), props.page)
        .await
    {
        Ok(c) => c,
        Err(_) => Vec::new(),
    };

    let last_message_id = match messages.first() {
        Some(l) => l.0.id.clone(),
        None => String::new(),
    };

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ChatTemplate {
            chat: chat.0,
            members: chat.1,
            messages,
            friends: database
                .auth
                .get_user_participating_relationships_of_status(
                    auth_user.id.clone(),
                    RelationshipStatus::Friends,
                )
                .await
                .unwrap(),
            last_message_id,
            is_helper,
            page: props.page,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct MessageTemplate {
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    message: (Message, Profile),
    is_helper: bool,
    is_own: bool,
}

/// GET /chats/_app/msg.html
pub async fn render_message_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(message): Json<(Message, Profile)>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(MessageTemplate {
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            is_own: auth_user.id == message.1.id,
            message,
            is_helper,
        }),
    })
}
