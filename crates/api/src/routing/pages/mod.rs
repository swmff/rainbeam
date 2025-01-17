use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Json;
use axum::{extract::State, Router};
use axum_extra::extract::CookieJar;

use ammonia::Builder;
use serde::{Deserialize, Serialize};
use authbeam::model::{
    IpBan, ItemStatus, ItemType, Notification, Permission, Profile, ProfileMetadata,
    RelationshipStatus,
};
use databeam::DefaultReturn;

use crate::database::Database;
use crate::model::{DatabaseError, FullResponse, Question, QuestionResponse, Reaction, ResponseComment};

use super::api;

pub mod chats;
pub mod circles;
pub mod mail;
pub mod market;
pub mod profile;
pub mod search;
pub mod settings;

#[derive(Serialize, Deserialize)]
struct HomepageTemplate {}

#[derive(Serialize, Deserialize)]
struct TimelineTemplate {
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
        return Json(DefaultReturn {
            success: true,
            message: String::new(),
            payload: crate::routing::into_some_serde_value(TimelineTemplate {
                friends: database
                    .auth
                    .get_user_participating_relationships_of_status(
                        ua.id.clone(),
                        RelationshipStatus::Friends,
                    )
                    .await
                    .unwrap(),
                page: props.page,
            }),
        });
    }

    // homepage
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(HomepageTemplate {}),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialTimelineTemplate {
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timelines/timeline.html
pub async fn partial_timeline_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PaginatedCleanQuery>,
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

    let responses_original = match database
        .get_responses_by_following_paginated(auth_user.id.to_owned(), props.page)
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let mut responses = Vec::new();
    if props.clean {
        for mut response in responses_original {
            response.0.author.clean();
            response.0.recipient.clean();
            response.1.author.clean();

            responses.push(response)
        }
    } else {
        responses = responses_original;
    }

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PartialTimelineTemplate {
            responses,
            relationships,
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialPublicTimelineTemplate {
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timelines/public_timeline.html
pub async fn partial_public_timeline_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PaginatedCleanQuery>,
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

    let responses_original = match database.get_responses_paginated(props.page).await {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let mut responses = Vec::new();
    if props.clean {
        for mut response in responses_original {
            response.0.author.clean();
            response.0.recipient.clean();
            response.1.author.clean();

            responses.push(response)
        }
    } else {
        responses = responses_original;
    }

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PartialPublicTimelineTemplate {
            responses,
            relationships,
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
pub struct MarkdownTemplate {
    title: String,
    text: String,
}

/// GET /site/about
pub async fn about_request(State(database): State<Database>) -> impl IntoResponse {
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(MarkdownTemplate {
            title: "About".to_string(),
            text: rainbeam_shared::fs::read(format!(
                "{}/site/about.md",
                database.config.static_dir
            ))
            .unwrap_or(database.config.description),
        }),
    })
}

/// GET /site/terms-of-service
pub async fn tos_request(State(database): State<Database>) -> impl IntoResponse {
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(MarkdownTemplate {
            title: "Terms of Service".to_string(),
            text: rainbeam_shared::fs::read(format!("{}/site/tos.md", database.config.static_dir))
                .unwrap_or(String::new()),
        }),
    })
}

/// GET /site/privacy
pub async fn privacy_request(State(database): State<Database>) -> impl IntoResponse {
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(MarkdownTemplate {
            title: "Privacy Policy".to_string(),
            text: rainbeam_shared::fs::read(format!(
                "{}/site/privacy.md",
                database.config.static_dir
            ))
            .unwrap_or(String::new()),
        }),
    })
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedQuery {
    #[serde(default)]
    pub page: i32,
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedCleanQuery {
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub clean: bool,
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

#[derive(Serialize, Deserialize)]
pub struct CleanProfileQuery {
    #[serde(default)]
    pub page: i32,
    pub tag: Option<String>,
    pub q: Option<String>,
    #[serde(default)]
    pub clean: bool,
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

#[derive(Serialize, Deserialize)]
struct QuestionTemplate {
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

    let question = match database.get_question(id.clone()).await {
        Ok(ua) => ua,
        Err(e) => return Json(e.to_json()),
    };

    let responses = match database.get_responses_by_question(id.to_owned()).await {
        Ok(responses) => responses,
        Err(_) => return Json(DatabaseError::Other.to_json()),
    };

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let reactions = match database.get_reactions_by_asset(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
    };

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(QuestionTemplate {
            already_responded: if let Some(ref ua) = auth_user {
                database
                    .get_response_by_question_and_author(id.clone(), ua.id.clone())
                    .await
                    .is_ok()
            } else {
                false
            },
            question,
            responses,
            is_powerful,
            is_helper,
            reactions,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialPostsTemplate {
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timelines/posts.html
pub async fn partial_posts_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PaginatedCleanQuery>,
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

    let responses_original = match database.get_posts_paginated(props.page).await {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let mut responses = Vec::new();
    if props.clean {
        for mut response in responses_original {
            response.0.author.clean();
            response.0.recipient.clean();
            response.1.author.clean();

            responses.push(response)
        }
    } else {
        responses = responses_original;
    }

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        if group.permissions.contains(&Permission::Helper) {
            is_helper = true
        }

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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PartialPostsTemplate {
            responses,
            relationships,
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialPostsFollowingTemplate {
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool,
    is_helper: bool,
}

/// GET /_app/timelines/posts_following.html
pub async fn partial_posts_following_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PaginatedCleanQuery>,
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

    let responses_original = match database
        .get_posts_by_following_paginated(props.page, auth_user.id.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let mut responses = Vec::new();
    if props.clean {
        for mut response in responses_original {
            response.0.author.clean();
            response.0.recipient.clean();
            response.1.author.clean();

            responses.push(response)
        }
    } else {
        responses = responses_original;
    }

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PartialPostsFollowingTemplate {
            responses,
            relationships,
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ResponseTemplate {
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

    let response = match database.get_response(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
    };

    let comments = match database
        .get_comments_by_response_paginated(id.clone(), query.page)
        .await
    {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
    };

    let reactions = match database.get_reactions_by_asset(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
    };

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ResponseTemplate {
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
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialResponseTemplate {
    response: FullResponse,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    is_powerful: bool,
    is_helper: bool,
    is_pinned: bool,
    show_comments: bool,
    show_pin_button: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PartialResponseProps {
    pub id: String,
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
        Err(e) => return Json(e.to_json()),
    };

    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PartialResponseTemplate {
            is_pinned: false,
            show_comments: true,
            show_pin_button: false,
            response,
            anonymous_username: Some("anonymous".to_string()), // TODO: fetch recipient setting
            anonymous_avatar: None,
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct CommentTemplate {
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

    let comment = match database.get_comment(id.clone(), true).await {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
    };

    let response = match database.get_response(comment.0.response.clone()).await {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
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
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let reactions = match database.get_reactions_by_asset(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(CommentTemplate {
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
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct InboxTemplate {
    unread: Vec<Question>,
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let unread = match database
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread,
        Err(_) => return Json(DatabaseError::Other.to_json()),
    };

    let is_helper: bool = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(InboxTemplate {
            unread,
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
                    .unwrap_or(&"/images/default-avatar.svg".to_string())
                    .to_string(),
            ),
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct GlobalTimelineTemplate {
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let questions = match database
        .get_global_questions_by_following_paginated(auth_user.id.clone(), query.page)
        .await
    {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
    };

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(GlobalTimelineTemplate {
            questions,
            relationships,
            is_helper,
            page: query.page,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PublicGlobalTimelineTemplate {
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

    let mut questions = match database.get_global_questions_paginated(query.page).await {
        Ok(r) => r,
        Err(e) => return Json(e.to_json()),
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
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PublicGlobalTimelineTemplate {
            questions,
            relationships,
            is_helper,
            page: query.page,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct NotificationsTemplate {
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
        Err(_) => return Json(DatabaseError::Other.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(NotificationsTemplate {
            notifs,
            page: props.page,
            pid,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ReportsTemplate {
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // check permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    if !group.permissions.contains(&Permission::Helper) {
        // we must be a manager to do this
        return Json(DatabaseError::NotAllowed.to_json());
    }

    // ...

    let reports = match database
        .auth
        .get_notifications_by_recipient("*".to_string())
        .await
    {
        Ok(r) => r,
        Err(_) => return Json(DatabaseError::Other.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ReportsTemplate { reports }),
    })
}

#[derive(Serialize, Deserialize)]
struct AuditTemplate {
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // check permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    if !group.permissions.contains(&Permission::Helper) {
        // we must be a manager to do this
        return Json(DatabaseError::NotAllowed.to_json());
    }

    // ...

    let logs = match database
        .auth
        .get_notifications_by_recipient_paginated("*(audit)".to_string(), props.page)
        .await
    {
        Ok(r) => r,
        Err(_) => return Json(DatabaseError::Other.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(AuditTemplate {
            logs,
            page: props.page,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct IpbansTemplate {
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // check permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    if !group.permissions.contains(&Permission::Helper) {
        // we must be a manager to do this
        return Json(DatabaseError::NotAllowed.to_json());
    }

    // ...
    let bans = match database.auth.get_ipbans(auth_user.clone()).await {
        Ok(r) => r,
        Err(_) => return Json(DatabaseError::Other.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(IpbansTemplate { bans }),
    })
}

// ...
pub async fn routes(database: Database) -> Router {
    Router::new()
        .route("/", get(homepage_request))
        .route("/site/about", get(about_request))
        .route("/site/terms-of-service", get(tos_request))
        .route("/site/privacy", get(privacy_request))
        // inbox
        .route("/inbox", get(inbox_request))
        .route("/inbox/global", get(public_global_timeline_request))
        .route("/inbox/global/following", get(global_timeline_request))
        .route("/inbox/notifications", get(notifications_request))
        .route("/inbox/reports", get(reports_request)) // staff
        .route("/inbox/audit", get(audit_log_request)) // staff
        .route("/inbox/audit/ipbans", get(ipbans_request)) // staff
        // assets
        .route("/@{username}/q/{id}", get(question_request))
        .route("/@{username}/r/{id}", get(response_request))
        .route("/@{username}/c/{id}", get(comment_request))
        // profiles
        .route("/@{username}/_app/warning", get(profile::warning_request))
        .route("/@{username}/comments", get(profile::comments_request))
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
            "/@{username}/_app/feed.html",
            get(profile::partial_profile_request),
        )
        .route(
            "/@{username}/_app/comments.html",
            get(profile::partial_comments_request),
        )
        .route("/@{username}", get(profile::profile_request))
        .route("/{id}", get(api::profiles::expand_request))
        // circles
        .route("/circles", get(circles::circles_request))
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
        .route("/inbox/mail/letter/{id}", get(mail::view_request))
        .route(
            "/inbox/mail/_app/components/mail.html",
            get(mail::partial_mail_request),
        )
        // market
        .route("/market", get(market::homepage_request))
        .route("/market/item/{id}", get(market::item_request))
        .route(
            "/market/_app/theme_playground.html/{id}",
            get(market::theme_playground_request),
        )
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
        .route(
            "/_app/timelines/posts_following.html",
            get(partial_posts_following_request),
        )
        // ...
        .with_state(database)
}
