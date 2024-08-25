use askama_axum::Template;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, response::Html};
use axum_extra::extract::CookieJar;

use xsu_authman::model::{Permission, Profile, UserFollow, Warning};

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, Question, QuestionResponse};

use super::{clean_metadata, PaginatedQuery, ProfileQuery};

#[derive(Template)]
#[template(path = "profile/profile.html")]
struct ProfileTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    responses: Vec<(Question, QuestionResponse, usize, usize)>,
    response_count: usize,
    questions_count: usize,
    followers_count: usize,
    following_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    pinned: Option<Vec<(Question, QuestionResponse, usize, usize)>>,
    page: i32,
    tag: String,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_self: bool,
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
            .get_follow(ua.id.to_owned(), other.id.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let is_following_you = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(other.id.clone(), ua.id.to_owned())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let responses = if let Some(ref tag) = query.tag {
        match database
            .get_responses_by_author_tagged_paginated(
                other.id.to_owned(),
                tag.to_owned(),
                query.page,
            )
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Html(e.to_html(database)),
        }
    } else {
        match database
            .get_responses_by_author_paginated(other.id.to_owned(), query.page)
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Html(e.to_html(database)),
        }
    };

    let pinned = if let Some(pinned) = other.metadata.kv.get("sparkler:pinned") {
        if pinned.is_empty() {
            None
        } else {
            let mut out = Vec::new();

            for id in pinned.split(",") {
                match database.get_response(id.to_string()).await {
                    Ok(response) => {
                        if response.1.author.id != other.id {
                            // don't allow us to pin responses from other users
                            continue;
                        }

                        out.push(response)
                    }
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

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    Html(
        ProfileTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            other: other.clone(),
            responses,
            response_count: database
                .get_response_count_by_author(other.id.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(other.id.clone())
                .await,
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
            pinned,
            page: query.page,
            tag: query.tag.unwrap_or(String::new()),
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
            hide_social: (other
                .metadata
                .kv
                .get("sparkler:private_social")
                .unwrap_or(&"false".to_string())
                == "true")
                && !is_self,
            is_blocked: if let Some(block_list) = other.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
            is_self,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "profile/followers.html")]
struct FollowersTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    response_count: usize,
    questions_count: usize,
    followers: Vec<(UserFollow, Profile, Profile)>,
    followers_count: usize,
    following_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    page: i32,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_self: bool,
}

/// GET /@:username/followers
pub async fn followers_request(
    jar: CookieJar,
    Path(username): Path<String>,
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
            .get_follow(ua.id.to_owned(), other.id.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let is_following_you = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(other.id.clone(), ua.id.to_owned())
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

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    if !is_self
        && (other
            .metadata
            .kv
            .get("sparkler:private_social")
            .unwrap_or(&"false".to_string())
            == "true")
    {
        // hide social if not self and private_social is true
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        FollowersTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            other: other.clone(),
            response_count: database
                .get_response_count_by_author(other.id.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(other.id.clone())
                .await,
            followers: database
                .auth
                .get_followers_paginated(other.id.clone(), query.page)
                .await
                .unwrap(),
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
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
            is_self,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "profile/following.html")]
struct FollowingTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    response_count: usize,
    questions_count: usize,
    followers_count: usize,
    following: Vec<(UserFollow, Profile, Profile)>,
    following_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    page: i32,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_self: bool,
}

/// GET /@:username/following
pub async fn following_request(
    jar: CookieJar,
    Path(username): Path<String>,
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
            .get_follow(ua.id.to_owned(), other.id.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let is_following_you = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(other.id.clone(), ua.id.to_owned())
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

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    if !is_self
        && (other
            .metadata
            .kv
            .get("sparkler:private_social")
            .unwrap_or(&"false".to_string())
            == "true")
    {
        // hide social if not self and private_social is true
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        FollowingTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            other: other.clone(),
            response_count: database
                .get_response_count_by_author(other.id.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(other.id.clone())
                .await,
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            following: database
                .auth
                .get_following_paginated(other.id.clone(), query.page)
                .await
                .unwrap(),
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
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
            is_self,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "profile/questions.html")]
struct ProfileQuestionsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    questions: Vec<(Question, usize, usize)>,
    questions_count: usize,
    response_count: usize,
    followers_count: usize,
    following_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    page: i32,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_self: bool,
}

/// GET /@:username/questions
pub async fn questions_request(
    jar: CookieJar,
    Path(username): Path<String>,
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
            .get_follow(ua.id.to_owned(), other.id.clone())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let is_following_you = if let Some(ref ua) = auth_user {
        match database
            .auth
            .get_follow(other.id.clone(), ua.id.to_owned())
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    };

    let questions = match database
        .get_global_questions_by_author_paginated(other.id.to_owned(), query.page)
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

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    Html(
        ProfileQuestionsTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            other: other.clone(),
            questions,
            questions_count: database
                .get_global_questions_count_by_author(other.id.clone())
                .await,
            response_count: database
                .get_response_count_by_author(other.id.clone())
                .await,
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
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
            hide_social: (other
                .metadata
                .kv
                .get("sparkler:private_social")
                .unwrap_or(&"false".to_string())
                == "true")
                && !is_self,
            is_blocked: if let Some(block_list) = other.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
            is_self,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "profile/warnings.html")]
struct WarningsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    other: Profile,
    warnings: Vec<Warning>,
    response_count: usize,
    questions_count: usize,
    followers_count: usize,
    following_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    // ...
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_self: bool,
}

/// GET /@:username/warnings
pub async fn warnings_request(
    jar: CookieJar,
    Path(username): Path<String>,
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
        .get_questions_by_recipient(auth_user.to_owned().id)
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = database
        .auth
        .get_notification_count_by_recipient(auth_user.to_owned().id)
        .await;

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let is_following = match database
        .auth
        .get_follow(auth_user.id.to_owned(), other.id.clone())
        .await
    {
        Ok(_) => true,
        Err(_) => false,
    };

    let is_following_you = match database
        .auth
        .get_follow(other.id.clone(), auth_user.id.to_owned())
        .await
    {
        Ok(_) => true,
        Err(_) => false,
    };

    let posting_as = auth_user.username.clone();

    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Manager)
    };

    if !is_powerful {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    let warnings = match database
        .auth
        .get_warnings_by_recipient(other.id.clone(), auth_user.clone())
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(DatabaseError::Other.to_html(database)),
    };

    let is_self = auth_user.id == other.id;

    Html(
        WarningsTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            other: other.clone(),
            warnings,
            response_count: database
                .get_response_count_by_author(other.id.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(other.id.clone())
                .await,
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
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
            hide_social: (other
                .metadata
                .kv
                .get("sparkler:private_social")
                .unwrap_or(&"false".to_string())
                == "true")
                && !is_self,
            is_blocked: if let Some(block_list) = other.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
            is_self,
        }
        .render()
        .unwrap(),
    )
}
