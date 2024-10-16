use askama_axum::Template;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_extra::extract::CookieJar;

use authbeam::model::Profile;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, Page};
use super::PaginatedQuery;

#[derive(Template)]
#[template(path = "pages/homepage.html")]
struct HomepageTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    pages: Vec<Page>,
    page: i32,
}

/// GET /pages
pub async fn pages_homepage_request(
    jar: CookieJar,
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

    if auth_user.tier < database.server_options.tiers.pages {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

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

    let pages = match database
        .get_pages_by_author_paginated(auth_user.id.clone(), props.page)
        .await
    {
        Ok(c) => c,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        HomepageTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            pages,
            page: props.page,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Serialize, Deserialize)]
pub struct EditorQuery {
    #[serde(default)]
    pub slug: String,
}

#[derive(Template)]
#[template(path = "pages/editor.html")]
struct EditorTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    page: Option<Page>,
}

/// GET /pages/editor
pub async fn page_editor_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<EditorQuery>,
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

    if auth_user.tier < database.server_options.tiers.pages {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

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

    Html(
        EditorTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            page: if !props.slug.is_empty() {
                match database
                    .get_page_by_owner_slug(auth_user.id, props.slug)
                    .await
                {
                    Ok(page) => Some(page),
                    Err(e) => return Html(e.to_html(database)),
                }
            } else {
                None
            },
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "pages/view.html")]
struct ViewTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    page: Page,
    other: Profile,
    is_self: bool,
}

/// GET /@:username/blog/:slug
pub async fn page_view_request(
    jar: CookieJar,
    Path((username, slug)): Path<(String, String)>,
    State(database): State<Database>,
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
        match database.get_questions_by_recipient(ua.id.to_owned()).await {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
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

    let other = match database.get_profile(username).await {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_html(database)),
    };

    let page = match database
        .get_page_by_owner_slug(other.id.clone(), slug)
        .await
    {
        Ok(page) => page,
        Err(e) => return Html(e.to_html(database)),
    };

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    Html(
        ViewTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            page,
            other,
            is_self,
        }
        .render()
        .unwrap(),
    )
}
