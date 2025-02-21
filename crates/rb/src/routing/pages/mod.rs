use std::collections::HashMap;

use reva_axum::Template;
use axum::extract::{Path, Query};
use axum::http::status::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{extract::State, response::Html, Router};
use axum_extra::extract::CookieJar;

use ammonia::Builder;
use langbeam::LangFile;
use serde::{Deserialize, Serialize};
use authbeam::model::{
    FinePermission, IpBan, ItemStatus, ItemType, Notification, Profile, ProfileMetadata,
    RelationshipStatus,
};

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, FullResponse, Question, QuestionResponse, Reaction, ResponseComment};
use crate::ToHtml;

use super::api;

pub mod chats;
pub mod circles;
pub mod mail;
pub mod market;
pub mod profile;
pub mod search;
pub mod settings;

/// Escape a username's characters if we are unable to find a "good" character
///
/// A "good" character is any alphanumeric character.
pub fn escape_username(name: &String) -> String {
    // comb through chars, if we never find anything that is actually a letter,
    // go ahead and escape
    let mut found_good: bool = false;

    for char in name.chars() {
        if char.is_alphanumeric() {
            found_good = true;
            break;
        }
    }

    if !found_good {
        return "bad username".to_string();
    }

    // return given data
    name.to_owned()
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub config: Config,
    pub lang: LangFile,
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
#[template(path = "homepage.html")]
struct HomepageTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
}

#[derive(Template)]
#[template(path = "timelines/timeline.html")]
struct TimelineTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    friends: Vec<(Box<Profile>, Box<Profile>)>,
    page: i32,
}

/// GET /
pub async fn homepage_request(
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
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    // timeline
    if let Some(ref ua) = auth_user {
        let unread = match database.get_questions_by_recipient(ua.id.to_owned()).await {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        };

        let notifs = database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await;

        // ...
        return Html(
            TimelineTemplate {
                config: database.config.clone(),
                lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                    c.value_trimmed()
                } else {
                    ""
                }),
                profile: auth_user.clone(),
                unread,
                notifs,
                friends: database
                    .auth
                    .get_user_participating_relationships_of_status(
                        ua.id.clone(),
                        RelationshipStatus::Friends,
                    )
                    .await
                    .unwrap(),
                page: props.page,
            }
            .render()
            .unwrap(),
        );
    }

    // homepage
    Html(
        HomepageTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "partials/timelines/timeline.html")]
struct PartialTimelineTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timelines/timeline.html
pub async fn partial_timeline_request(
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

    let responses = match database
        .get_responses_by_following_paginated(auth_user.id.to_owned(), props.page)
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        if group.permissions.check_helper() {
            is_helper = true
        }

        group.permissions.check_manager()
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    for response in &responses {
        if relationships.contains_key(&response.1.author.id) {
            continue;
        }

        if is_helper {
            // make sure staff can view your responses
            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
            continue;
        }

        if response.1.author.id == auth_user.id {
            // make sure we can view our own responses
            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
            continue;
        };

        relationships.insert(
            response.1.author.id.clone(),
            database
                .auth
                .get_user_relationship(response.1.author.id.clone(), auth_user.id.clone())
                .await
                .0,
        );
    }

    // ...
    return Html(
        PartialTimelineTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            responses,
            relationships,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    );
}

#[derive(Template)]
#[template(path = "timelines/public_timeline.html")]
struct PublicTimelineTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    page: i32,
}

/// GET /public
pub async fn public_timeline_request(
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
            Err(_) => return Html(DatabaseError::NotAllowed.to_string()),
        },
        None => return Html(DatabaseError::NotAllowed.to_string()),
    };

    // timeline
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

    // ...
    return Html(
        PublicTimelineTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            notifs,
            page: props.page,
        }
        .render()
        .unwrap(),
    );
}

#[derive(Template)]
#[template(path = "partials/timelines/timeline.html")]
struct PartialPublicTimelineTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timelines/public_timeline.html
pub async fn partial_public_timeline_request(
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

    let responses = match database.get_responses_paginated(props.page).await {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        if group.permissions.check_helper() {
            is_helper = true
        }

        group.permissions.check_manager()
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    for response in &responses {
        if relationships.contains_key(&response.1.author.id) {
            continue;
        }

        if is_helper {
            // make sure staff can view your responses
            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
            continue;
        }

        if response.1.author.id == auth_user.id {
            // make sure we can view our own responses
            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
            continue;
        };

        relationships.insert(
            response.1.author.id.clone(),
            database
                .auth
                .get_user_relationship(response.1.author.id.clone(), auth_user.id.clone())
                .await
                .0,
        );
    }

    // ...
    return Html(
        PartialPublicTimelineTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            responses,
            relationships,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    );
}

#[derive(Template)]
#[template(path = "general_markdown_text.html")]
pub struct MarkdownTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    title: String,
    text: String,
}

/// GET /site/about
pub async fn about_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
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

    Html(
        MarkdownTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            title: "About".to_string(),
            text: rainbeam_shared::fs::read(format!(
                "{}/site/about.md",
                database.config.static_dir
            ))
            .unwrap_or(database.config.description),
        }
        .render()
        .unwrap(),
    )
}

/// GET /site/terms-of-service
pub async fn tos_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
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

    Html(
        MarkdownTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            title: "Terms of Service".to_string(),
            text: rainbeam_shared::fs::read(format!("{}/site/tos.md", database.config.static_dir))
                .unwrap_or(String::new()),
        }
        .render()
        .unwrap(),
    )
}

/// GET /site/privacy
pub async fn privacy_request(
    jar: CookieJar,
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

    Html(
        MarkdownTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            title: "Privacy Policy".to_string(),
            text: rainbeam_shared::fs::read(format!(
                "{}/site/privacy.md",
                database.config.static_dir
            ))
            .unwrap_or(String::new()),
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "fun/carp.html")]
struct CarpTemplate {}

/// GET /site/fun/carp
pub async fn carp_request() -> impl IntoResponse {
    Html(CarpTemplate {}.render().unwrap())
}

#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
}

/// GET /login
pub async fn login_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
            Err(_) => None,
        },
        None => None,
    };

    Html(
        LoginTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "auth/sign_up.html")]
struct SignUpTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
}

/// GET /sign_up
pub async fn sign_up_request(
    jar: CookieJar,
    State(database): State<Database>,
) -> impl IntoResponse {
    if database.config.registration_enabled == false {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    // ...
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
            Err(_) => None,
        },
        None => None,
    };

    Html(
        SignUpTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedQuery {
    #[serde(default)]
    pub page: i32,
}

#[derive(Serialize, Deserialize)]
pub struct NotificationsQuery {
    #[serde(default)]
    page: i32,
    #[serde(default)]
    profile: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    page: i32,
    #[serde(default)]
    q: String,
    #[serde(default)]
    tag: String,
}

#[derive(Serialize, Deserialize)]
pub struct MarketQuery {
    #[serde(default)]
    page: i32,
    #[serde(default)]
    q: String,
    #[serde(default)]
    status: ItemStatus,
    #[serde(default)]
    creator: String,
    #[serde(default)]
    r#type: Option<ItemType>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchHomeQuery {
    #[serde(default)]
    driver: i8,
}

#[derive(Serialize, Deserialize)]
pub struct ProfileQuery {
    #[serde(default)]
    pub page: i32,
    pub tag: Option<String>,
    pub q: Option<String>,
}

/// Escape profile colors
pub fn color_escape(color: &&&String) -> String {
    remove_tags(
        &color
            .replace(";", "")
            .replace("<", "&lt;")
            .replace(">", "%gt;")
            .replace("}", "")
            .replace("{", "")
            .replace("url(\"", "url(\"/api/v0/util/ext/image?img=")
            .replace("url('", "url('/api/v0/util/ext/image?img=")
            .replace("url(https://", "url(/api/v0/util/ext/image?img=https://"),
    )
}

/// Clean profile metadata
pub fn remove_tags(input: &str) -> String {
    Builder::default()
        .rm_tags(&["img", "a", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6"])
        .clean(input)
        .to_string()
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("</script>", "</not-script")
}

/// Clean profile metadata
pub fn clean_metadata(metadata: &ProfileMetadata) -> String {
    remove_tags(&serde_json::to_string(&clean_metadata_raw(metadata)).unwrap())
}

/// Clean profile metadata
pub fn clean_metadata_raw(metadata: &ProfileMetadata) -> ProfileMetadata {
    // remove stupid characters
    let mut metadata = metadata.to_owned();

    for field in metadata.kv.clone() {
        metadata.kv.insert(
            field.0.to_string(),
            field
                .1
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("url(\"", "url(\"/api/v0/util/ext/image?img=")
                .replace("url(https://", "url(/api/v0/util/ext/image?img=https://")
                .replace("<style>", "")
                .replace("</style>", ""),
        );
    }

    // ...
    metadata
}

/// Clean profile metadata short
pub fn clean_metadata_short(metadata: &ProfileMetadata) -> String {
    remove_tags(&serde_json::to_string(&clean_metadata_short_raw(metadata)).unwrap())
        .replace("\u{200d}", "")
}

/// Clean profile metadata short row
pub fn clean_metadata_short_raw(metadata: &ProfileMetadata) -> ProfileMetadata {
    // remove stupid characters
    let mut metadata = metadata.to_owned();

    for field in metadata.kv.clone() {
        metadata.kv.insert(
            field.0.to_string(),
            field
                .1
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("<style>", "")
                .replace("</style>", ""),
        );
    }

    // ...
    metadata
}

#[derive(Template)]
#[template(path = "views/question.html")]
struct QuestionTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    question: Question,
    responses: Vec<FullResponse>,
    reactions: Vec<Reaction>,
    already_responded: bool,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /@{}/q/{id}
pub async fn question_request(
    jar: CookieJar,
    Path((_, id)): Path<(String, String)>,
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

    let question = match database.get_question(id.clone()).await {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_html(database)),
    };

    let responses = match database.get_responses_by_question(id.to_owned()).await {
        Ok(responses) => responses,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
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

    // ...
    Html(
        QuestionTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            already_responded: if let Some(ref ua) = auth_user {
                database
                    .get_response_by_question_and_author(id.clone(), ua.id.clone())
                    .await
                    .is_ok()
            } else {
                false
            },
            profile: auth_user,
            unread,
            notifs,
            question,
            responses,
            is_powerful,
            is_helper,
            reactions,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "timelines/posts.html")]
struct PublicPostsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    page: i32,
}

/// GET /inbox/posts
pub async fn public_posts_timeline_request(
    jar: CookieJar,
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

    let mut responses = match database.get_posts_paginated(query.page).await {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    // remove content from blocked users/users that have blocked us
    if let Some(ref ua) = auth_user {
        let blocked = match database
            .auth
            .get_user_relationships_of_status(ua.id.clone(), RelationshipStatus::Blocked)
            .await
        {
            Ok(l) => l,
            Err(_) => Vec::new(),
        };

        for user in blocked {
            for (i, _) in responses
                .clone()
                .iter()
                .filter(|x| (x.1.author.id == user.0.id) | (x.0.author.id == user.0.id))
                .enumerate()
            {
                if responses.get(i).is_some() {
                    responses.remove(i);
                }
            }
        }
    }

    // ...
    Html(
        PublicPostsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            unread,
            notifs,
            page: query.page,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "partials/timelines/posts.html")]
struct PartialPostsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timelines/posts.html
pub async fn partial_posts_request(
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
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let mut responses = match database.get_posts_paginated(props.page).await {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    // remove content from blocked users/users that have blocked us
    if let Some(ref ua) = auth_user {
        let blocked = match database
            .auth
            .get_user_relationships_of_status(ua.id.clone(), RelationshipStatus::Blocked)
            .await
        {
            Ok(l) => l,
            Err(_) => Vec::new(),
        };

        for user in blocked {
            for (i, _) in responses
                .clone()
                .iter()
                .filter(|x| x.1.author.id == user.0.id)
                .enumerate()
            {
                if i > responses.len() {
                    continue;
                }

                responses.remove(i);
            }
        }
    }

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        if group.permissions.check_helper() {
            is_helper = true
        }

        group.permissions.check_manager()
    } else {
        false
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    if let Some(ref ua) = auth_user {
        for response in &responses {
            if relationships.contains_key(&response.1.author.id) {
                continue;
            }

            if is_helper {
                // make sure staff can view your responses
                relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
                continue;
            }

            if response.1.author.id == ua.id {
                // make sure we can view our own responses
                relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
                continue;
            };

            relationships.insert(
                response.1.author.id.clone(),
                database
                    .auth
                    .get_user_relationship(response.1.author.id.clone(), ua.id.clone())
                    .await
                    .0,
            );
        }
    } else {
        // the posts timeline requires that we have an entry for every relationship,
        // since we don't have an account every single relationship should be unknown
        for response in &responses {
            if relationships.contains_key(&response.1.author.id) {
                continue;
            }

            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Unknown);
        }
    }

    // ...
    return Html(
        PartialPostsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
            responses,
            relationships,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    );
}

#[derive(Template)]
#[template(path = "timelines/posts_following.html")]
struct FollowingPostsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    page: i32,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /inbox/posts/following
pub async fn following_posts_timeline_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<PaginatedQuery>,
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

    let responses = match database
        .get_posts_by_following_paginated(query.page, auth_user.id.clone())
        .await
    {
        Ok(responses) => responses,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    for response in &responses {
        if relationships.contains_key(&response.1.author.id) {
            continue;
        }

        if response.1.author.id == auth_user.id {
            // make sure we can view our own responses
            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
            continue;
        };

        relationships.insert(
            response.1.author.id.clone(),
            database
                .auth
                .get_user_relationship(response.1.author.id.clone(), auth_user.id.clone())
                .await
                .0,
        );
    }

    // ...
    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        if group.permissions.check_helper() {
            is_helper = true
        }

        group.permissions.check_manager()
    };

    // ...
    Html(
        FollowingPostsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            notifs,
            page: query.page,
            responses,
            relationships,
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

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

#[derive(Serialize, Deserialize)]
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
            is_powerful,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "inbox.html")]
struct InboxTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: Vec<Question>,
    notifs: usize,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    is_helper: bool,
}

/// GET /inbox
pub async fn inbox_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
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
        Ok(unread) => unread,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.id.to_owned())
        .await;

    let is_helper: bool = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    Html(
        InboxTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            unread,
            notifs,
            anonymous_username: Some(
                auth_user
                    .metadata
                    .kv
                    .get("sparkler:anonymous_username")
                    .unwrap_or(&"anonymous".to_string())
                    .to_string(),
            ),
            anonymous_avatar: Some(
                auth_user
                    .metadata
                    .kv
                    .get("sparkler:anonymous_avatar")
                    .unwrap_or(&"/static/images/default-avatar.svg".to_string())
                    .to_string(),
            ),
            profile: Some(auth_user),
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "timelines/global_question_timeline.html")]
struct GlobalTimelineTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    questions: Vec<(Question, usize, usize)>,
    relationships: HashMap<String, RelationshipStatus>,
    is_helper: bool,
    page: i32,
}

/// GET /inbox/global/following
pub async fn global_timeline_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<PaginatedQuery>,
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

    let questions = match database
        .get_global_questions_by_following_paginated(auth_user.id.clone(), query.page)
        .await
    {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    for question in &questions {
        if relationships.contains_key(&question.0.author.id) {
            continue;
        }

        if is_helper {
            // make sure staff can view your questions
            relationships.insert(question.0.author.id.clone(), RelationshipStatus::Friends);
            continue;
        }

        if question.0.author.id == auth_user.id {
            // make sure we can view our own responses
            relationships.insert(question.0.author.id.clone(), RelationshipStatus::Friends);
            continue;
        };

        relationships.insert(
            question.0.author.id.clone(),
            database
                .auth
                .get_user_relationship(question.0.author.id.clone(), auth_user.id.clone())
                .await
                .0,
        );
    }

    // ...
    Html(
        GlobalTimelineTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            notifs,
            questions,
            relationships,
            is_helper,
            page: query.page,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "timelines/public_global_question_timeline.html")]
struct PublicGlobalTimelineTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    questions: Vec<(Question, usize, usize)>,
    relationships: HashMap<String, RelationshipStatus>,
    is_helper: bool,
    page: i32,
}

/// GET /inbox/global
pub async fn public_global_timeline_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<PaginatedQuery>,
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

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    let mut questions = match database.get_global_questions_paginated(query.page).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    // remove content from blocked users/users that have blocked us
    let blocked = match database
        .auth
        .get_user_relationships_of_status(auth_user.id.clone(), RelationshipStatus::Blocked)
        .await
    {
        Ok(l) => l,
        Err(_) => Vec::new(),
    };

    for user in blocked {
        for (i, _) in questions
            .clone()
            .iter()
            .filter(|x| x.0.author.id == user.0.id)
            .enumerate()
        {
            questions.remove(i);
        }
    }

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    for question in &questions {
        if relationships.contains_key(&question.0.author.id) {
            continue;
        }

        if is_helper {
            // make sure staff can view your questions
            relationships.insert(question.0.author.id.clone(), RelationshipStatus::Friends);
            continue;
        }

        if question.0.author.id == auth_user.id {
            // make sure we can view our own responses
            relationships.insert(question.0.author.id.clone(), RelationshipStatus::Friends);
            continue;
        };

        relationships.insert(
            question.0.author.id.clone(),
            database
                .auth
                .get_user_relationship(question.0.author.id.clone(), auth_user.id.clone())
                .await
                .0,
        );
    }

    // ...
    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    Html(
        PublicGlobalTimelineTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            notifs,
            questions,
            relationships,
            is_helper,
            page: query.page,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "intents/post.html")]
struct ComposeTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
}

/// GET /_app/components/compose.html
pub async fn compose_request(
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

    Html(
        ComposeTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "notifications.html")]
struct NotificationsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: Vec<Notification>,
    page: i32,
    pid: String,
}

/// GET /inbox/notifications
pub async fn notifications_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<NotificationsQuery>,
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

    let unread = match database
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let pid = if is_helper && !props.profile.is_empty() {
        // use the given profile value if we gave one and we are a helper
        props.profile
    } else {
        // otherwise, use the current user
        auth_user.id.clone()
    };

    let notifs = match database
        .auth
        .get_notifications_by_recipient_paginated(pid.clone(), props.page)
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    Html(
        NotificationsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            notifs,
            page: props.page,
            pid,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "reports.html")]
struct ReportsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    reports: Vec<Notification>,
}

/// GET /inbox/reports
pub async fn reports_request(
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

    // check permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    if !group.permissions.check(FinePermission::VIEW_REPORTS) {
        // we must be a manager to do this
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    // ...
    let unread = match database
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let reports = match database
        .auth
        .get_notifications_by_recipient("*".to_string())
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    Html(
        ReportsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            reports,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "audit.html")]
struct AuditTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    logs: Vec<Notification>,
    page: i32,
}

/// GET /inbox/audit
pub async fn audit_log_request(
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

    // check permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    if !group.permissions.check(FinePermission::VIEW_AUDIT_LOG) {
        // we must be a manager to do this
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    // ...
    let unread = match database
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let logs = match database
        .auth
        .get_notifications_by_recipient_paginated("*(audit)".to_string(), props.page)
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    Html(
        AuditTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            logs,
            page: props.page,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "ipbans.html")]
struct IpbansTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    bans: Vec<IpBan>,
}

/// GET /inbox/audit/ipbans
pub async fn ipbans_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
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

    // check permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    if !group.permissions.check(FinePermission::BAN_IP) {
        // we must be a manager to do this
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    // ...
    let unread = match database
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let bans = match database.auth.get_ipbans(auth_user.clone()).await {
        Ok(r) => r,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    Html(
        IpbansTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            bans,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "intents/report.html")]
struct ReportTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
}

/// GET /intents/report
pub async fn report_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
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

    Html(
        ReportTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: auth_user,
        }
        .render()
        .unwrap(),
    )
}

// ...
pub async fn routes(database: Database) -> Router {
    Router::new()
        .route("/", get(homepage_request))
        .route("/public", get(public_timeline_request))
        .route("/site/about", get(about_request))
        .route("/site/terms-of-service", get(tos_request))
        .route("/site/privacy", get(privacy_request))
        .route("/intents/report", get(report_request))
        .route("/site/fun/carp", get(carp_request))
        // inbox
        .route("/inbox", get(inbox_request))
        .route("/inbox/posts", get(public_posts_timeline_request))
        .route(
            "/inbox/posts/following",
            get(following_posts_timeline_request),
        )
        .route("/inbox/global", get(public_global_timeline_request))
        .route("/inbox/global/following", get(global_timeline_request))
        .route("/inbox/notifications", get(notifications_request))
        .route("/inbox/reports", get(reports_request)) // staff
        .route("/inbox/audit", get(audit_log_request)) // staff
        .route("/inbox/audit/ipbans", get(ipbans_request)) // staff
        .route("/intents/post", get(compose_request))
        // assets
        .route("/@{username}/q/{id}", get(question_request))
        .route("/@{username}/r/{id}", get(response_request))
        .route("/@{username}/c/{id}", get(comment_request))
        // profiles
        .route("/@{username}/_app/warning", get(profile::warning_request))
        .route("/@{username}/mod", get(profile::mod_request)) // staff
        .route("/@{username}/questions", get(profile::questions_request))
        .route("/@{username}/questions/inbox", get(profile::inbox_request)) // staff
        .route(
            "/@{username}/questions/outbox",
            get(profile::outbox_request),
        ) // staff
        .route("/@{username}/following", get(profile::following_request))
        .route("/@{username}/followers", get(profile::followers_request))
        .route("/@{username}/friends", get(profile::friends_request))
        .route(
            "/@{username}/friends/requests",
            get(profile::friend_requests_request),
        )
        .route("/@{username}/friends/blocks", get(profile::blocks_request))
        .route("/@{username}/embed", get(profile::profile_embed_request))
        .route(
            "/@{username}/relationship/friend_accept",
            get(profile::friend_request),
        )
        .route(
            "/@{username}/_app/card.html",
            get(profile::render_card_request),
        )
        .route(
            "/@{username}/_app/feed.html",
            get(profile::partial_profile_request),
        )
        .route(
            "/@{username}/layout",
            get(profile::profile_layout_editor_request),
        )
        .route("/@{username}", get(profile::profile_request))
        .route("/{id}", get(api::profiles::expand_request))
        // circles
        .route("/circles", get(circles::circles_request))
        .route("/circles/new", get(circles::new_circle_request))
        .route(
            "/circles/@{name}/settings/privacy",
            get(circles::privacy_settings_request),
        )
        .route(
            "/circles/@{name}/settings",
            get(circles::general_settings_request),
        )
        .route(
            "/circles/@{name}/memberlist/accept",
            get(circles::accept_invite_request),
        )
        .route(
            "/circles/@{name}/memberlist",
            get(circles::memberlist_request),
        )
        .route("/circles/@{name}", get(circles::profile_redirect_request))
        .route(
            "/+{name}/_app/feed.html",
            get(circles::partial_profile_request),
        )
        .route("/+{name}", get(circles::profile_request))
        // settings
        .route("/settings", get(settings::account_settings))
        .route("/settings/sessions", get(settings::sessions_settings))
        .route("/settings/profile", get(settings::profile_settings))
        .route("/settings/privacy", get(settings::privacy_settings))
        .route("/settings/coins", get(settings::coins_settings))
        // search
        .route("/search", get(search::search_homepage_request))
        .route("/search/responses", get(search::search_responses_request))
        .route("/search/posts", get(search::search_posts_request))
        .route("/search/questions", get(search::search_questions_request))
        .route("/search/users", get(search::search_users_request))
        // chats
        .route("/chats", get(chats::chats_homepage_request))
        .route("/chats/{id}", get(chats::chat_request))
        .route("/chats/_app/msg.html", post(chats::render_message_request))
        // mail
        .route("/inbox/mail", get(mail::inbox_request))
        .route("/inbox/mail/sent", get(mail::outbox_request))
        .route("/inbox/mail/compose", get(mail::compose_request))
        .route("/inbox/mail/letter/{id}", get(mail::view_request))
        .route(
            "/inbox/mail/_app/components/mail.html",
            get(mail::partial_mail_request),
        )
        // market
        .route("/market", get(market::homepage_request))
        .route("/market/new", get(market::create_request))
        .route("/market/item/{id}", get(market::item_request))
        .route(
            "/market/_app/theme_playground.html/{id}",
            get(market::theme_playground_request),
        )
        .route(
            "/market/_app/layout_playground.html/{id}",
            get(market::layout_playground_request),
        )
        // auth
        .route("/login", get(login_request))
        .route("/sign_up", get(sign_up_request))
        // expanders
        .route("/+q/{id}", get(api::questions::expand_request))
        .route("/question/{id}", get(api::questions::expand_request))
        .route("/+r/{id}", get(api::responses::expand_request))
        .route("/response/{id}", get(api::responses::expand_request))
        .route("/+c/{id}", get(api::comments::expand_request))
        .route("/comment/{id}", get(api::comments::expand_request))
        .route("/+u/{id}", get(api::profiles::expand_request))
        .route("/+i/{ip}", get(api::profiles::expand_ip_request))
        .route("/+g/{id}", get(api::circles::expand_request))
        // partials
        .route(
            "/_app/components/response.html",
            get(partial_response_request),
        )
        .route(
            "/_app/timelines/timeline.html",
            get(partial_timeline_request),
        )
        .route(
            "/_app/timelines/public_timeline.html",
            get(partial_public_timeline_request),
        )
        .route("/_app/timelines/posts.html", get(partial_posts_request))
        // ...
        .with_state(database)
}
