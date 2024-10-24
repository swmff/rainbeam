use std::collections::HashMap;

use askama_axum::Template;
use axum::extract::{Path, Query};
use axum::http::status::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{extract::State, response::Html, Router};
use axum_extra::extract::CookieJar;

use ammonia::Builder;
use serde::{Deserialize, Serialize};
use authbeam::model::{
    Notification, Permission, Profile, ProfileMetadata, RelationshipStatus, UserFollow,
};

use crate::config::Config;
use crate::database::Database;
use crate::model::{
    DatabaseError, FullResponse, Question, QuestionResponse, Reaction, ResponseComment,
    ResponseReply,
};

use super::api;

mod chats;
mod circles;
mod pages;
mod profile;
mod search;
mod settings;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub config: Config,
    pub profile: Option<Profile>,
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
    profile: Option<Profile>,
}

#[derive(Template)]
#[template(path = "timelines/timeline.html")]
struct TimelineTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    friends: Vec<(Profile, Profile)>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /
pub async fn homepage_request(
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

        let responses = match database.get_responses_by_following(ua.id.to_owned()).await {
            Ok(responses) => responses,
            Err(e) => return Html(e.to_html(database)),
        };

        let mut is_helper: bool = false;
        let is_powerful = if let Some(ref ua) = auth_user {
            let group = match database.auth.get_group_by_id(ua.group).await {
                Ok(g) => g,
                Err(_) => return Html(DatabaseError::Other.to_html(database)),
            };

            is_helper = group.permissions.contains(&Permission::Helper);
            group.permissions.contains(&Permission::Manager)
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
            for response in &responses {
                // no user, no relationships
                if relationships.contains_key(&response.1.author.id) {
                    continue;
                }

                relationships.insert(response.1.author.id.clone(), RelationshipStatus::Unknown);
            }
        }

        // ...
        return Html(
            TimelineTemplate {
                config: database.server_options,
                profile: auth_user.clone(),
                unread,
                notifs,
                responses,
                relationships,
                friends: database
                    .auth
                    .get_user_participating_relationships_of_status(
                        ua.id.clone(),
                        RelationshipStatus::Friends,
                    )
                    .await
                    .unwrap(),
                is_powerful,
                is_helper,
            }
            .render()
            .unwrap(),
        );
    }

    // homepage
    Html(
        HomepageTemplate {
            config: database.server_options,
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
    profile: Option<Profile>,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timeline.html
pub async fn partial_timeline_request(
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

    let responses = match database
        .get_responses_by_following(auth_user.id.to_owned())
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

        if group.permissions.contains(&Permission::Helper) {
            is_helper = true
        }

        group.permissions.contains(&Permission::Manager)
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
            config: database.server_options,
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
#[template(path = "about.html")]
struct AboutTemplate {
    config: Config,
    profile: Option<Profile>,
    about: String,
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
        AboutTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            about: shared::fs::read(format!(
                "{}/site/about.md",
                database.server_options.static_dir
            ))
            .unwrap_or(database.server_options.description),
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
    profile: Option<Profile>,
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
            config: database.server_options,
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
    profile: Option<Profile>,
}

/// GET /sign_up
pub async fn sign_up_request(
    jar: CookieJar,
    State(database): State<Database>,
) -> impl IntoResponse {
    if database.server_options.registration_enabled == false {
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
            config: database.server_options,
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
pub struct SearchHomeQuery {
    #[serde(default)]
    driver: i8,
}

#[derive(Serialize, Deserialize)]
pub struct ProfileQuery {
    #[serde(default)]
    page: i32,
    tag: Option<String>,
    q: Option<String>,
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
            .replace("url(\"", "url(\"/api/util/ext/image?img=")
            .replace("url('", "url('/api/util/ext/image?img=")
            .replace("url(https://", "url(/api/util/ext/image?img=https://"),
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
                .replace("url(\"", "url(\"/api/util/ext/image?img=")
                .replace("url(https://", "url(/api/util/ext/image?img=https://")
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
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    question: Question,
    responses: Vec<FullResponse>,
    reactions: Vec<Reaction>,
    already_responded: bool,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /question/:id
pub async fn question_request(
    jar: CookieJar,
    Path(id): Path<String>,
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

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let reactions = match database.get_reactions_by_asset(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        QuestionTemplate {
            config: database.server_options.clone(),
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
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    page: i32,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
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
    let blocked = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_user_relationships_of_status(ua.id.clone(), RelationshipStatus::Blocked)
            .await
        {
            Ok(l) => l,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    for user in blocked {
        for (i, _) in responses
            .clone()
            .iter()
            .filter(|x| x.1.author.id == user.0.id)
            .enumerate()
        {
            responses.remove(i);
        }
    }

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
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
        for response in &responses {
            // no user, no relationships
            if relationships.contains_key(&response.1.author.id) {
                continue;
            }

            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Unknown);
        }
    }

    // ...
    Html(
        PublicPostsTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
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
#[template(path = "partials/timelines/posts.html")]
struct PartialPostsTemplate {
    config: Config,
    profile: Option<Profile>,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/posts.html
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
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let mut responses = match database.get_posts_paginated(props.page).await {
        Ok(responses) => responses,
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
        for (i, _) in responses
            .clone()
            .iter()
            .filter(|x| x.1.author.id == user.0.id)
            .enumerate()
        {
            responses.remove(i);
        }
    }

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        if group.permissions.contains(&Permission::Helper) {
            is_helper = true
        }

        group.permissions.contains(&Permission::Manager)
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
        PartialPostsTemplate {
            config: database.server_options,
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
#[template(path = "timelines/posts_following.html")]
struct FollowingPostsTemplate {
    config: Config,
    profile: Option<Profile>,
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

        if group.permissions.contains(&Permission::Helper) {
            is_helper = true
        }

        group.permissions.contains(&Permission::Manager)
    };

    Html(
        FollowingPostsTemplate {
            config: database.server_options.clone(),
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
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    question: Question,
    response: QuestionResponse,
    relationship: RelationshipStatus,
    comments: Vec<(ResponseComment, usize, usize)>,
    reply: ResponseReply,
    reactions: Vec<Reaction>,
    tags: String,
    page: i32,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /response/:id
pub async fn response_request(
    jar: CookieJar,
    Path(id): Path<String>,
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

    let response = match database.get_response(id.clone(), false).await {
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

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
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
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            question: response.0,
            tags: serde_json::to_string(&response.1.tags).unwrap(),
            response: response.1,
            relationship,
            reply: response.4,
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
#[template(path = "views/comment.html")]
struct CommentTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    comment: (ResponseComment, usize, usize),
    replies: Vec<(ResponseComment, usize, usize)>,
    reactions: Vec<Reaction>,
    page: i32,
    question: Question,
    response: QuestionResponse,
    reply: ResponseReply,
    reaction_count: usize,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /comment/:id
pub async fn comment_request(
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

    let response = match database
        .get_response(comment.0.response.clone(), false)
        .await
    {
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

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let reactions = match database.get_reactions_by_asset(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        CommentTemplate {
            config: database.server_options.clone(),
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
            reply: response.4,
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
    profile: Option<Profile>,
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

        group.permissions.contains(&Permission::Helper)
    };

    Html(
        InboxTemplate {
            config: database.server_options,
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
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    questions: Vec<(Question, usize, usize)>,
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

        group.permissions.contains(&Permission::Helper)
    };

    Html(
        GlobalTimelineTemplate {
            config: database.server_options,
            profile: Some(auth_user),
            unread,
            notifs,
            questions,
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
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    questions: Vec<(Question, usize, usize)>,
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

    // ...
    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Helper)
    };

    Html(
        PublicGlobalTimelineTemplate {
            config: database.server_options,
            profile: Some(auth_user),
            unread,
            notifs,
            questions,
            is_helper,
            page: query.page,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "compose.html")]
struct ComposeTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    following: Vec<(UserFollow, Profile, Profile)>,
}

/// GET /inbox/compose
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
        ComposeTemplate {
            config: database.server_options,
            following: database
                .auth
                .get_following(auth_user.id.clone())
                .await
                .unwrap_or(Vec::new()),
            profile: Some(auth_user),
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "notifications.html")]
struct NotificationsTemplate {
    config: Config,
    profile: Option<Profile>,
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

        group.permissions.contains(&Permission::Helper)
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
            config: database.server_options.clone(),
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
    profile: Option<Profile>,
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

    if !group.permissions.contains(&Permission::Helper) {
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
            config: database.server_options.clone(),
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
    profile: Option<Profile>,
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

    if !group.permissions.contains(&Permission::Helper) {
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
            config: database.server_options.clone(),
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
#[template(path = "report.html")]
struct ReportTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
}

/// GET /site/report
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

    Html(
        ReportTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

// ...
pub async fn routes(database: Database) -> Router {
    Router::new()
        .route("/", get(homepage_request))
        .route("/site/about", get(about_request))
        .route("/site/report", get(report_request))
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
        .route("/inbox/compose", get(compose_request))
        .route("/inbox/notifications", get(notifications_request))
        .route("/inbox/reports", get(reports_request)) // staff
        .route("/inbox/audit", get(audit_log_request)) // staff
        // assets
        .route("/question/:id", get(question_request))
        .route("/response/:id", get(response_request))
        .route("/comment/:id", get(comment_request))
        // profiles
        .route("/@:username/mod", get(profile::mod_request)) // staff
        .route("/@:username/questions", get(profile::questions_request))
        .route("/@:username/questions/inbox", get(profile::inbox_request)) // staff
        .route("/@:username/questions/outbox", get(profile::outbox_request)) // staff
        .route("/@:username/following", get(profile::following_request))
        .route("/@:username/followers", get(profile::followers_request))
        .route("/@:username/friends", get(profile::friends_request))
        .route("/@:username/embed", get(profile::profile_embed_request))
        .route(
            "/@:username/relationship/friend_accept",
            get(profile::friend_request),
        )
        .route(
            "/@:username/_app/card.html",
            get(profile::render_card_request),
        )
        .route(
            "/@:username/_app/feed.html",
            get(profile::partial_profile_request),
        )
        .route("/@:username", get(profile::profile_request))
        // circles
        .route("/circles", get(circles::circles_request))
        .route("/circles/new", get(circles::new_circle_request))
        .route(
            "/circles/@:name/settings/privacy",
            get(circles::privacy_settings_request),
        )
        .route(
            "/circles/@:name/settings",
            get(circles::profile_settings_request),
        )
        .route("/circles/@:name/inbox", get(circles::inbox_request))
        .route(
            "/circles/@:name/memberlist/accept",
            get(circles::accept_invite_request),
        )
        .route(
            "/circles/@:name/memberlist",
            get(circles::memberlist_request),
        )
        .route("/circles/@:name", get(circles::profile_redirect_request))
        .route("/+:name", get(circles::profile_request))
        // settings
        .route("/settings", get(settings::account_settings))
        .route("/settings/sessions", get(settings::sessions_settings))
        .route("/settings/profile", get(settings::profile_settings))
        .route("/settings/privacy", get(settings::privacy_settings))
        .route("/settings/system", get(settings::system_settings))
        // search
        .route("/search", get(search::search_homepage_request))
        .route("/search/responses", get(search::search_responses_request))
        .route("/search/posts", get(search::search_posts_request))
        .route("/search/questions", get(search::search_questions_request))
        .route("/search/users", get(search::search_users_request))
        // chats
        .route("/chats", get(chats::chats_homepage_request))
        .route("/chats/:id", get(chats::chat_request))
        .route("/chats/_app/msg.html", post(chats::render_message_request))
        // pages
        .route("/pages", get(pages::pages_homepage_request))
        .route("/pages/editor", get(pages::page_editor_request))
        .route("/@:username/blog/:slug", get(pages::page_view_request))
        // auth
        .route("/login", get(login_request))
        .route("/sign_up", get(sign_up_request))
        // expanders
        .route("/+q/:id", get(api::questions::expand_request))
        .route("/+r/:id", get(api::responses::expand_request))
        .route("/+c/:id", get(api::comments::expand_request))
        .route("/+u/:id", get(api::profiles::expand_request))
        .route("/+i/:ip", get(api::profiles::expand_ip_request))
        .route("/+p/:id", get(api::pages::expand_request))
        // partials
        .route("/_app/timeline.html", get(partial_timeline_request))
        .route("/_app/posts.html", get(partial_posts_request))
        // ...
        .with_state(database)
}
