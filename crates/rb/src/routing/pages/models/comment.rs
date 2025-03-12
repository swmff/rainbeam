use rainbeam::{
    database::Database,
    model::{ResponseComment, Question, QuestionResponse, Reaction, DatabaseError},
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
#[template(path = "views/comment.html")]
struct CommentTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    comment: (ResponseComment, usize, usize),
    replies: Vec<(ResponseComment, usize, usize)>,
    reactions: Vec<Reaction>,
    page: i32,
    question: Question,
    response: QuestionResponse,
    reaction_count: usize,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    open_replies_in_tab: bool,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /@{username}/c/{id}
pub async fn comment_request(
    jar: CookieJar,
    Path((_, id)): Path<(String, String)>,
    State(database): State<Database>,
    Query(props): Query<PaginatedQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let unread = if let Some(ref ua) = auth_user {
        database.get_inbox_count_by_recipient(&ua.id).await
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(&ua.id)
            .await
    } else {
        0
    };

    let comment = match database.get_comment(id.clone(), true).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let response = match database.get_response(comment.0.response.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let replies = match database
        .get_replies_by_comment_paginated(comment.0.id.clone(), props.page.clone())
        .await
    {
        Ok(r) => r,
        Err(_) => Vec::new(),
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

    let reactions = match database.get_reactions_by_asset(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        CommentTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            unread,
            notifs,
            comment,
            replies,
            reactions,
            page: props.page,
            question: response.0,
            response: response.1,
            reaction_count: response.3,
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
#[template(path = "partials/views/comments.html")]
struct CommentsPartialTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    response: QuestionResponse,
    comments: Vec<(ResponseComment, usize, usize)>,
    open_replies_in_tab: bool,
    is_powerful: bool,
    is_helper: bool,
}

#[derive(Deserialize)]
pub struct PartialCommentsProps {
    pub id: String,
    pub page: i32,
}

/// GET /_app/components/comments.html
pub async fn partial_comments_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PartialCommentsProps>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let comment = match database.get_comment(props.id.clone(), true).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let comments = match database
        .get_replies_by_comment_paginated(comment.0.id.clone(), props.page.clone())
        .await
    {
        Ok(r) => r,
        Err(_) => Vec::new(),
    };

    let response = match database.get_response(comment.0.response.clone()).await {
        Ok(r) => r.1,
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

    Html(
        CommentsPartialTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            response,
            comments,
            open_replies_in_tab: true,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "partials/views/comments.html")]
struct ResponseCommentsPartialTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    response: QuestionResponse,
    comments: Vec<(ResponseComment, usize, usize)>,
    open_replies_in_tab: bool,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/components/response_comments.html
pub async fn partial_response_comments_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PartialCommentsProps>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let response = match database.get_response(props.id.clone()).await {
        Ok(r) => r.1,
        Err(e) => return Html(e.to_html(database)),
    };

    let comments = match database
        .get_comments_by_response_paginated(response.id.clone(), props.page.clone())
        .await
    {
        Ok(r) => r,
        Err(_) => Vec::new(),
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

    Html(
        ResponseCommentsPartialTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            response,
            comments,
            open_replies_in_tab: true,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}
