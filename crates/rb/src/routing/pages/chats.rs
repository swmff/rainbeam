use reva_axum::Template;
use axum::response::IntoResponse;
use axum::Json;
use axum::{
    extract::{Query, State, Path},
    response::Html,
};
use axum_extra::extract::CookieJar;

use authbeam::model::{Profile, RelationshipStatus};

use crate::config::Config;
use crate::database::Database;
use crate::model::{Chat, DatabaseError, Message};
use super::PaginatedQuery;
use crate::ToHtml;

#[derive(Template)]
#[template(path = "chats/homepage.html")]
struct HomepageTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = match database
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.id.to_owned())
        .await;

    let chats = match database.get_chats_for_user(auth_user.id.clone()).await {
        Ok(c) => c,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        HomepageTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            chats,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "chats/chat.html")]
struct ChatTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = match database
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.id.to_owned())
        .await;

    let chat = match database.get_chat(id.clone()).await {
        Ok(c) => c,
        Err(e) => return Html(e.to_html(database)),
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
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    Html(
        ChatTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
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
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "chats/components/message.html")]
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
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    Html(
        MessageTemplate {
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            is_own: auth_user.id == message.1.id,
            message,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}
