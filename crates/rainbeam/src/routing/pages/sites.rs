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
use rainbeam::leafml::CompileHTML;

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, Site};
use super::PaginatedQuery;
use crate::ToHtml;

#[derive(Template)]
#[template(path = "sites/homepage.html")]
struct HomepageTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    sites: Vec<Site>,
    page: i32,
}

/// GET /site
pub async fn sites_homepage_request(
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

    let sites = match database
        .get_sites_by_author_paginated(auth_user.id.clone(), props.page)
        .await
    {
        Ok(c) => c,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        HomepageTemplate {
            config: database.server_options.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            sites,
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
#[template(path = "sites/editor.html")]
struct EditorTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Profile>,
    site_content: rainbeam::leafml::Plant,
    site: Option<Site>,
}

/// GET /sites/editor
pub async fn site_editor_request(
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

    let site = if !props.slug.is_empty() {
        match database.get_site_by_slug(props.slug).await {
            Ok(site) => Some(site),
            Err(e) => return Html(e.to_html(database)),
        }
    } else {
        None
    };

    Html(
        EditorTemplate {
            config: database.server_options.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            site_content: if let Some(ref site) = site {
                site.content.clone()
            } else {
                rainbeam::leafml::Plant::default()
            },
            site,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "sites/view.html")]
struct ViewTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Profile>,
    site: Site,
    html: String,
}

/// GET /:slug
pub async fn site_view_request(
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

    let site = match database.get_site_by_slug(slug).await {
        Ok(page) => page,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        ViewTemplate {
            config: database.server_options.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user.clone(),
            html: site.content.clone().compile(),
            site,
        }
        .render()
        .unwrap(),
    )
}
