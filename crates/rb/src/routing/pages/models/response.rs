use rainbeam::{
    database::Database,
    model::{
        ResponseComment, RelationshipStatus, Question, QuestionResponse, Reaction, FullResponse,
        DatabaseError,
    },
};
use rainbeam_shared::config::Config;
use authbeam::model::Profile;

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;
use reva_axum::Template;

use crate::{ToHtml, routing::pages::PaginatedQuery};
use serde::Deserialize;

#[derive(Template)]
#[template(path = "views/response.html")]
struct ResponseTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    question: Question,
    response: QuestionResponse,
    relationship: RelationshipStatus,
    comments: Vec<(ResponseComment, usize, usize)>,
    reactions: Vec<Reaction>,
    tags: String,
    page: i32,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    open_replies_in_tab: bool,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /@{}/r/{id}
pub async fn response_request(
    jar: CookieJar,
    Path((_, id)): Path<(String, String)>,
    State(database): State<Database>,
    Query(query): Query<PaginatedQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let unread = if let Some(ref ua) = auth_user {
        database
            .get_inbox_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    let response = match database.get_response(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let comments = match database
        .get_comments_by_response_paginated(id.clone(), query.page)
        .await
    {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let reactions = match database.get_reactions_by_asset(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        is_helper = group.permissions.check_helper();
        group.permissions.check_manager()
    } else {
        false
    };

    // get_relationship
    let mut relationship = RelationshipStatus::Unknown;

    if let Some(ref ua) = auth_user {
        if (response.1.author.id == ua.id) | is_helper {
            // make sure we can view our own responses
            relationship = RelationshipStatus::Friends;
        } else {
            relationship = database
                .auth
                .get_user_relationship(response.1.author.id.clone(), ua.id.clone())
                .await
                .0;
        }
    }

    // ...
    Html(
        ResponseTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user.clone(),
            unread,
            notifs,
            question: response.0,
            tags: serde_json::to_string(&response.1.tags).unwrap(),
            response: response.1.clone(),
            relationship,
            comments,
            reactions,
            page: query.page,
            anonymous_username: Some("anonymous".to_string()), // TODO: fetch recipient setting
            anonymous_avatar: None,
            open_replies_in_tab: false,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "components/response.html")]
struct PartialResponseTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    response: FullResponse,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    is_powerful: bool,
    is_helper: bool,
    do_not_render_question: bool,
    is_pinned: bool,
    show_comments: bool,
    show_pin_button: bool,
    do_render_nested: bool,
}

#[derive(Deserialize)]
pub struct PartialResponseProps {
    pub id: String,
    pub do_render_nested: bool,
}

/// GET /_app/components/response.html
pub async fn partial_response_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PartialResponseProps>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let response = match database.get_response(props.id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        is_helper = group.permissions.check_helper();
        group.permissions.check_manager()
    } else {
        false
    };

    // ...
    Html(
        PartialResponseTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            do_not_render_question: response.1.context.is_post,
            is_pinned: false,
            show_comments: true,
            show_pin_button: false,
            do_render_nested: props.do_render_nested,
            response,
            anonymous_username: Some("anonymous".to_string()), // TODO: fetch recipient setting
            anonymous_avatar: None,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}
