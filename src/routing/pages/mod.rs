use askama_axum::Template;
use axum::debug_handler;
use axum::extract::{Path, Query};
use axum::http::status::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{extract::State, response::Html, Router};
use axum_extra::extract::CookieJar;

use ammonia::Builder;
use serde::{Deserialize, Serialize};
use xsu_authman::model::{Notification, Permission, Profile, ProfileMetadata, UserFollow};

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, Question, QuestionResponse, Reaction, ResponseComment};

mod circles;
mod profile;
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
#[template(path = "timeline.html")]
struct TimelineTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    responses: Vec<(Question, QuestionResponse, usize, usize)>,
    is_powerful: bool,
}

/// GET /
#[debug_handler]
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

        let is_powerful = if let Some(ref ua) = auth_user {
            let group = match database.auth.get_group_by_id(ua.group).await {
                Ok(g) => g,
                Err(_) => return Html(DatabaseError::Other.to_html(database)),
            };

            group.permissions.contains(&Permission::Manager)
        } else {
            false
        };

        return Html(
            TimelineTemplate {
                config: database.server_options,
                profile: auth_user,
                unread,
                notifs,
                responses,
                is_powerful,
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

/// Escape profile colors
pub fn color_escape(color: &&&String) -> String {
    color
        .replace(";", "")
        .replace("<", "&lt;")
        .replace(">", "%gt;")
        .replace("}", "")
        .replace("{", "")
        .replace("url(\"", "url(\"/api/util/ext/image?img=")
}

/// Clean profile metadata
pub fn clean_metadata(metadata: &ProfileMetadata) -> String {
    Builder::default()
        .rm_tags(&["img", "a", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6"])
        .clean(&serde_json::to_string(&metadata).unwrap())
        .to_string()
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[derive(Template)]
#[template(path = "global_question.html")]
struct GlobalQuestionTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    question: Question,
    responses: Vec<(Question, QuestionResponse, usize, usize)>,
    already_responded: bool,
    is_powerful: bool,
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
        GlobalQuestionTemplate {
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
    question: Question,
    response: QuestionResponse,
    comments: Vec<ResponseComment>,
    reactions: Vec<Reaction>,
    page: i32,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    is_powerful: bool,
    has_reacted: bool,
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

    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let has_reacted = if let Some(ref ua) = auth_user {
        database.get_reaction(ua.id.clone(), id).await.is_ok()
    } else {
        false
    };

    Html(
        ResponseTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            question: response.0,
            response: response.1,
            comments,
            reactions,
            page: query.page,
            anonymous_username: Some("anonymous".to_string()), // TODO: fetch recipient setting
            anonymous_avatar: None,
            is_powerful,
            has_reacted,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "comment.html")]
struct CommentTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    comment: ResponseComment,
    question: Question,
    response: QuestionResponse,
    comment_count: usize,
    reaction_count: usize,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    is_powerful: bool,
}

/// GET /comment/:id
pub async fn comment_request(
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

    let comment = match database.get_comment(id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let response = match database.get_response(comment.response.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
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
        CommentTemplate {
            config: database.server_options.clone(),
            profile: auth_user,
            unread,
            notifs,
            comment,
            question: response.0,
            response: response.1,
            comment_count: response.2,
            reaction_count: response.3,
            anonymous_username: Some("anonymous".to_string()), // TODO: fetch recipient setting
            anonymous_avatar: None,
            is_powerful,
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
    is_powerful: bool,
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
        .get_global_questions_by_following(auth_user.id.clone())
        .await
    {
        Ok(r) => r,
        Err(e) => return Html(e.to_html(database)),
    };

    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Manager)
    };

    Html(
        GlobalTimelineTemplate {
            config: database.server_options,
            profile: Some(auth_user),
            unread,
            notifs,
            questions,
            is_powerful,
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
        .get_questions_by_recipient(auth_user.id.to_owned())
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = match database
        .auth
        .get_notifications_by_recipient(auth_user.id.to_owned())
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
        // inbox
        .route("/inbox", get(inbox_request))
        .route("/inbox/global", get(global_timeline_request))
        .route("/inbox/compose", get(compose_request))
        .route("/inbox/notifications", get(notifications_request))
        .route("/inbox/reports", get(reports_request)) // staff
        // assets
        .route("/question/:id", get(global_question_request))
        .route("/response/:id", get(response_request))
        .route("/comment/:id", get(comment_request))
        // profiles
        .route("/@:username/warnings", get(profile::warnings_request)) // staff
        .route("/@:username/questions", get(profile::questions_request))
        .route("/@:username/following", get(profile::following_request))
        .route("/@:username/followers", get(profile::followers_request))
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
        // auth
        .route("/login", get(login_request))
        .route("/sign_up", get(sign_up_request))
        // ...
        .with_state(database)
}
