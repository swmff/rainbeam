use askama_axum::Template;
use axum::{
    extract::{Query, State, Path},
    response::{Html, IntoResponse},
    Router,
    routing::get,
};
use axum_extra::extract::CookieJar;

use authbeam::model::Profile;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, Post};
use crate::ToHtml;

pub async fn routes(database: Database) -> Router {
    Router::new()
        .route("/", get(post_editor_request))
        .route("/me/posts", get(posts_homepage_request))
        // ...
        .with_state(database)
}

pub async fn rb_external(database: rb::database::Database) -> Router {
    Router::new()
        .route("/login", get(rb::routing::pages::login_request))
        .route("/sign_up", get(rb::routing::pages::sign_up_request))
        // ...
        .with_state(database)
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedQuery {
    #[serde(default)]
    pub page: i32,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub config: Config,
    pub lang: langbeam::LangFile,
    pub profile: Option<Box<Profile>>,
    pub message: String,
}

pub async fn not_found(State(database): State<Database>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(DatabaseError::NotFound.to_html(database)),
    )
}

#[derive(Template)]
#[template(path = "blogbeam/homepage.html")]
struct HomepageTemplate {
    config: Config,
    profile: Option<Box<Profile>>,
    posts: Vec<Post>,
    page: i32,
}

/// GET /me/posts
pub async fn posts_homepage_request(
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

    let posts = match database
        .get_posts_by_author_paginated(auth_user.id.clone(), props.page)
        .await
    {
        Ok(c) => c,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        HomepageTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            posts,
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
#[template(path = "blogbeam/editor.html")]
struct EditorTemplate {
    config: Config,
    profile: Option<Box<Profile>>,
    post: Option<Post>,
}

/// GET /
pub async fn post_editor_request(
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

    Html(
        EditorTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            post: if !props.slug.is_empty() {
                match database.get_post_by_slug(props.slug).await {
                    Ok(post) => Some(post),
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
#[template(path = "blogbeam/view.html")]
struct ViewTemplate {
    config: Config,
    profile: Option<Box<Profile>>,
    post: Post,
    is_self: bool,
}

/// GET /:slug
pub async fn posts_view_request(
    jar: CookieJar,
    Path(slug): Path<String>,
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

    let post = match database.get_post_by_slug(slug).await {
        Ok(post) => post,
        Err(e) => return Html(e.to_html(database)),
    };

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == post.owner
    } else {
        false
    };

    Html(
        ViewTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            post,
            is_self,
        }
        .render()
        .unwrap(),
    )
}
