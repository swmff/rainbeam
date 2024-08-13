use askama_axum::Template;
use axum::extract::{Path, Query};
use axum::http::status::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{extract::State, response::Html, Router};
use axum_extra::extract::CookieJar;

use serde::{Deserialize, Serialize};
use xsu_authman::model::{Notification, Permission, Profile, UserFollow};

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, Question, QuestionResponse, ResponseComment};

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
#[template(path = "timeline.html")]
struct TimelineTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    responses: Vec<(QuestionResponse, usize)>,
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
        let unread = match database
            .get_questions_by_recipient(ua.username.to_owned())
            .await
        {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        };

        let notifs = database
            .auth
            .get_notification_count_by_recipient(ua.username.to_owned())
            .await;

        let responses = match database
            .get_responses_by_following(ua.username.to_owned())
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Html(e.to_html(database)),
        };

        return Html(
            TimelineTemplate {
                config: database.server_options,
                profile: auth_user,
                unread,
                notifs,
                responses,
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
            about: xsu_util::fs::read(format!(
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
#[template(path = "login.html")]
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
#[template(path = "sign_up.html")]
struct SignUpTemplate {
    config: Config,
    profile: Option<Profile>,
}

/// GET /sign_up
pub async fn sign_up_request(
    jar: CookieJar,
    State(database): State<Database>,
) -> impl IntoResponse {
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
pub struct ProfileQuery {
    #[serde(default)]
    page: i32,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    responses: Vec<(QuestionResponse, usize)>,
    response_count: usize,
    questions_count: usize,
    followers_count: usize,
    following_count: usize,
    is_following: bool,
    metadata: String,
    pinned: Option<Vec<(QuestionResponse, usize)>>,
    page: i32,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool, // if you are a site manager
}

/// GET /@:username
pub async fn profile_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Query(query): Query<ProfileQuery>,
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
        match database
            .get_questions_by_recipient(ua.username.to_owned())
            .await
        {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.username.to_owned())
            .await
    } else {
        0
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let is_following = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(ua.username.to_owned(), other.username.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let responses = match database
        .get_responses_by_author_paginated(other.username.to_owned(), query.page)
        .await
    {
        Ok(responses) => responses,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    let pinned = if let Some(pinned) = other.metadata.kv.get("sparkler:pinned") {
        if pinned.is_empty() {
            None
        } else {
            let mut out = Vec::new();

            for id in pinned.split(",") {
                match database.get_response(id.to_string()).await {
                    Ok(response) => out.push(response),
                    Err(_) => continue,
                }
            }

            Some(out)
        }
    } else {
        None
    };

    let posting_as = if let Some(ref ua) = auth_user {
        ua.username.clone()
    } else {
        "anonymous".to_string()
    };

    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    Html(
        ProfileTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            other: other.clone(),
            responses,
            response_count: database
                .get_response_count_by_author(username.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(username.clone())
                .await,
            followers_count: database.auth.get_followers_count(username.clone()).await,
            following_count: database.auth.get_following_count(username.clone()).await,
            is_following,
            metadata: serde_json::to_string(&other.metadata).unwrap(),
            pinned,
            page: query.page,
            // ...
            lock_profile: other
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: other
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: other
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = other.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "followers.html")]
struct FollowersTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    response_count: usize,
    questions_count: usize,
    followers: Vec<UserFollow>,
    followers_count: usize,
    following_count: usize,
    is_following: bool,
    metadata: String,
    page: i32,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
}

/// GET /@:username/followers
pub async fn followers_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Query(query): Query<ProfileQuery>,
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
        match database
            .get_questions_by_recipient(ua.username.to_owned())
            .await
        {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.username.to_owned())
            .await
    } else {
        0
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_string()),
    };

    let is_following = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(ua.username.to_owned(), other.username.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let posting_as = if let Some(ref ua) = auth_user {
        ua.username.clone()
    } else {
        "anonymous".to_string()
    };

    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    Html(
        FollowersTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            other: other.clone(),
            response_count: database
                .get_response_count_by_author(username.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(username.clone())
                .await,
            followers: database
                .auth
                .get_followers_paginated(username.clone(), query.page)
                .await
                .unwrap_or(Vec::new()),
            followers_count: database.auth.get_followers_count(username.clone()).await,
            following_count: database.auth.get_following_count(username.clone()).await,
            is_following,
            metadata: serde_json::to_string(&other.metadata).unwrap(),
            page: query.page,
            // ...
            lock_profile: other
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: other
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: other
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = other.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "following.html")]
struct FollowingTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    response_count: usize,
    questions_count: usize,
    followers_count: usize,
    following: Vec<UserFollow>,
    following_count: usize,
    is_following: bool,
    metadata: String,
    page: i32,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
}

/// GET /@:username/following
pub async fn following_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Query(query): Query<ProfileQuery>,
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
        match database
            .get_questions_by_recipient(ua.username.to_owned())
            .await
        {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.username.to_owned())
            .await
    } else {
        0
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_string()),
    };

    let is_following = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(ua.username.to_owned(), other.username.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let posting_as = if let Some(ref ua) = auth_user {
        ua.username.clone()
    } else {
        "anonymous".to_string()
    };

    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    Html(
        FollowingTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            other: other.clone(),
            response_count: database
                .get_response_count_by_author(username.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(username.clone())
                .await,
            followers_count: database.auth.get_followers_count(username.clone()).await,
            following_count: database.auth.get_following_count(username.clone()).await,
            following: database
                .auth
                .get_following_paginated(username, query.page)
                .await
                .unwrap_or(Vec::new()),
            is_following,
            metadata: serde_json::to_string(&other.metadata).unwrap(),
            page: query.page,
            // ...
            lock_profile: other
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: other
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: other
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = other.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "profile_questions.html")]
struct ProfileQuestionsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    questions: Vec<(Question, i32)>,
    questions_count: usize,
    response_count: usize,
    followers_count: usize,
    following_count: usize,
    is_following: bool,
    metadata: String,
    page: i32,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
}

/// GET /@:username/questions
pub async fn profile_questions_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Query(query): Query<ProfileQuery>,
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
        match database
            .get_questions_by_recipient(ua.username.to_owned())
            .await
        {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.username.to_owned())
            .await
    } else {
        0
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_string()),
    };

    let is_following = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(ua.username.to_owned(), other.username.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let questions = match database
        .get_global_questions_by_author_paginated(other.username.to_owned(), query.page)
        .await
    {
        Ok(responses) => responses,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    let posting_as = if let Some(ref ua) = auth_user {
        ua.username.clone()
    } else {
        "anonymous".to_string()
    };

    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    Html(
        ProfileQuestionsTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            other: other.clone(),
            questions,
            questions_count: database
                .get_global_questions_count_by_author(username.clone())
                .await,
            response_count: database
                .get_response_count_by_author(username.clone())
                .await,
            followers_count: database.auth.get_followers_count(username.clone()).await,
            following_count: database.auth.get_following_count(username.clone()).await,
            is_following,
            metadata: serde_json::to_string(&other.metadata).unwrap(),
            page: query.page,
            // ...
            lock_profile: other
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: other
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: other
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = other.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "global_question.html")]
struct GlobalQuestionTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    question: Question,
    responses: Vec<(QuestionResponse, usize)>,
}

/// GET /question/:id
pub async fn global_question_request(
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
        match database
            .get_questions_by_recipient(ua.username.to_owned())
            .await
        {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.username.to_owned())
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

    Html(
        GlobalQuestionTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            question,
            responses,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "response.html")]
struct ResponseTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    response: QuestionResponse,
    comments: Vec<ResponseComment>,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
}

/// GET /response/:id
pub async fn response_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Query(query): Query<ProfileQuery>,
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
        match database
            .get_questions_by_recipient(ua.username.to_owned())
            .await
        {
            Ok(unread) => unread.len(),
            Err(_) => 0,
        }
    } else {
        0
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.username.to_owned())
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

    Html(
        ResponseTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            response: response.0,
            comments,
            anonymous_username: Some("anonymous".to_string()), // TODO: fetch recipient setting
            anonymous_avatar: None,
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
        .get_questions_by_recipient(auth_user.username.to_owned())
        .await
    {
        Ok(unread) => unread,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.username.to_owned())
        .await;

    Html(
        InboxTemplate {
            config: database.server_options,
            profile: Some(auth_user),
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "global_question_timeline.html")]
struct GlobalTimelineTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    questions: Vec<(Question, i32)>,
}

/// GET /inbox/global
pub async fn global_timeline_request(
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
        .get_questions_by_recipient(auth_user.username.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.username.to_owned())
        .await;

    let questions = match database
        .get_global_questions_by_following(auth_user.username.clone())
        .await
    {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    Html(
        GlobalTimelineTemplate {
            config: database.server_options,
            profile: Some(auth_user),
            unread,
            notifs,
            questions,
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
    following: Vec<UserFollow>,
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
        .get_questions_by_recipient(auth_user.username.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.username.to_owned())
        .await;

    Html(
        ComposeTemplate {
            config: database.server_options,
            following: database
                .auth
                .get_following(auth_user.username.clone())
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
#[template(path = "account_settings.html")]
struct AccountSettingsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    metadata: String,
}

/// GET /settings
pub async fn account_settings_request(
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
        .get_questions_by_recipient(auth_user.username.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.username.to_owned())
        .await;

    Html(
        AccountSettingsTemplate {
            config: database.server_options,
            metadata: serde_json::to_string(&auth_user.metadata).unwrap(),
            profile: Some(auth_user),
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "profile_settings.html")]
struct ProfileSettingsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    metadata: String,
}

/// GET /settings/profile
pub async fn profile_settings_request(
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
        .get_questions_by_recipient(auth_user.username.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.username.to_owned())
        .await;

    Html(
        ProfileSettingsTemplate {
            config: database.server_options,
            metadata: serde_json::to_string(&auth_user.metadata).unwrap(),
            profile: Some(auth_user),
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "privacy_settings.html")]
struct PrivacySettingsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    metadata: String,
}

/// GET /settings/privacy
pub async fn privacy_settings_request(
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
        .get_questions_by_recipient(auth_user.username.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.username.to_owned())
        .await;

    Html(
        PrivacySettingsTemplate {
            config: database.server_options,
            metadata: serde_json::to_string(&auth_user.metadata).unwrap(),
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
}

/// GET /inbox/notifications
pub async fn notifications_request(
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
        .get_questions_by_recipient(auth_user.username.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = match database
        .auth
        .get_notifications_by_recipient(auth_user.username.to_owned())
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

    if !group.permissions.contains(&Permission::Manager) {
        // we must be a manager to do this
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    // ...
    let unread = match database
        .get_questions_by_recipient(auth_user.username.to_owned())
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

// ...
pub async fn routes(database: Database) -> Router {
    Router::new()
        .route("/", get(homepage_request))
        .route("/site/about", get(about_request))
        // inbox
        .route("/inbox", get(inbox_request))
        .route("/inbox/global", get(global_timeline_request))
        .route("/inbox/compose", get(compose_request))
        .route("/inbox/notifications", get(notifications_request))
        .route("/inbox/reports", get(reports_request)) // staff
        // profiles
        .route("/question/:id", get(global_question_request))
        .route("/response/:id", get(response_request))
        .route("/@:username/questions", get(profile_questions_request))
        .route("/@:username/following", get(following_request))
        .route("/@:username/followers", get(followers_request))
        .route("/@:username", get(profile_request))
        // settings
        .route("/settings", get(account_settings_request))
        .route("/settings/profile", get(profile_settings_request))
        .route("/settings/privacy", get(privacy_settings_request))
        // auth
        .route("/login", get(login_request))
        .route("/sign_up", get(sign_up_request))
        // ...
        .with_state(database)
}
