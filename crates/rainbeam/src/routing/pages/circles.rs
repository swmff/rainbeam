use ammonia::Builder;
use askama_axum::Template;
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Redirect};
use axum::{extract::State, response::Html};
use axum_extra::extract::CookieJar;

use authbeam::model::{Permission, Profile};

use crate::config::Config;
use crate::database::Database;
use crate::model::{Circle, CircleMetadata, DatabaseError, FullResponse, MembershipStatus, Question};

use super::PaginatedQuery;

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

/// Clean circle metadata
pub fn clean_metadata(metadata: &CircleMetadata) -> String {
    remove_tags(&serde_json::to_string(&clean_metadata_raw(metadata)).unwrap())
}

/// Clean circle metadata
pub fn clean_metadata_raw(metadata: &CircleMetadata) -> CircleMetadata {
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
#[template(path = "circle/homepage.html")]
struct CirclesTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    circles: Vec<Circle>,
}

/// GET /circles
pub async fn circles_request(
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

    Html(
        CirclesTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            circles: match database.get_user_circle_memberships(auth_user.id).await {
                Ok(c) => c,
                Err(e) => return Html(e.to_html(database)),
            },
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "circle/new.html")]
struct NewCircleTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
}

/// GET /circles/new
pub async fn new_circle_request(
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

    Html(
        NewCircleTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

pub async fn profile_redirect_request(Path(name): Path<String>) -> impl IntoResponse {
    Redirect::to(&format!("/+{name}"))
}

#[derive(Template)]
#[template(path = "circle/profile.html")]
struct ProfileTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    inbox_count: usize,
    notifs: usize,
    circle: Circle,
    responses: Vec<FullResponse>,
    response_count: usize,
    member_count: usize,
    metadata: String,
    pinned: Option<Vec<FullResponse>>,
    page: i32,
    // ...
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_helper: bool,
    is_member: bool,
    is_owner: bool,
}

/// GET /+:name
pub async fn profile_request(
    jar: CookieJar,
    Path(name): Path<String>,
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let inbox_count = match database
        .get_questions_by_recipient(format!("circle:{}", circle.id))
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    let mut responses = match database
        .get_responses_by_circle_paginated(circle.id.to_owned(), query.page)
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
    };

    let pinned = if let Some(pinned) = circle.metadata.kv.get("sparkler:pinned") {
        if pinned.is_empty() {
            None
        } else {
            let mut out = Vec::new();

            for id in pinned.split(",") {
                match database.get_response(id.to_string(), false).await {
                    Ok(response) => {
                        // TODO: check author circle membership status
                        // remove from responses
                        let in_responses = responses.iter().position(|r| r.1.id == response.1.id);

                        if let Some(index) = in_responses {
                            responses.remove(index);
                        };

                        // push
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

    let mut is_owner = false;
    let is_member = if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Active
    } else {
        false
    };

    Html(
        ProfileTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            inbox_count,
            circle: circle.clone(),
            responses,
            response_count: database
                .get_response_count_by_circle(circle.id.clone())
                .await,
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            pinned,
            page: query.page,
            // ...
            layout: circle
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
            lock_profile: circle
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: circle
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: circle
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = circle.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
            is_helper,
            is_member,
            is_owner,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "circle/memberlist.html")]
struct MemberlistTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    inbox_count: usize,
    notifs: usize,
    circle: Circle,
    members: Vec<Profile>,
    response_count: usize,
    member_count: usize,
    metadata: String,
    // ...
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_member: bool,
    is_owner: bool,
}

/// GET /circles/@:name/memberlist
pub async fn memberlist_request(
    jar: CookieJar,
    Path(name): Path<String>,
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let inbox_count = match database
        .get_questions_by_recipient(format!("circle:{}", circle.id))
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
    };

    let members = match database.get_circle_memberships(circle.id.to_owned()).await {
        Ok(responses) => responses,
        Err(e) => return Html(e.to_html(database)),
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

    let mut is_owner = false;
    let is_member = if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Active
    } else {
        false
    };

    Html(
        MemberlistTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            inbox_count,
            circle: circle.clone(),
            members,
            response_count: database
                .get_response_count_by_circle(circle.id.clone())
                .await,
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            // ...
            layout: circle
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
            lock_profile: circle
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: circle
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: circle
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = circle.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
            is_member,
            is_owner,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "circle/accept_invite.html")]
struct AcceptInviteTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    inbox_count: usize,
    notifs: usize,
    circle: Circle,
    response_count: usize,
    member_count: usize,
    metadata: String,
    // ...
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_member: bool,
    is_owner: bool,
}

/// GET /circles/@:name/memberlist/accept
pub async fn accept_invite_request(
    jar: CookieJar,
    Path(name): Path<String>,
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let inbox_count = match database
        .get_questions_by_recipient(format!("circle:{}", circle.id))
        .await
    {
        Ok(unread) => unread.len(),
        Err(_) => 0,
    };

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
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

    let mut is_owner = false;
    let is_member = if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Active
    } else {
        false
    };

    if is_member {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        AcceptInviteTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            inbox_count,
            circle: circle.clone(),
            response_count: database
                .get_response_count_by_circle(circle.id.clone())
                .await,
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            // ...
            layout: circle
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
            lock_profile: circle
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: circle
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: circle
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = circle.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
            is_member,
            is_owner,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "circle/inbox.html")]
struct InboxTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    questions: Vec<Question>,
    notifs: usize,
    circle: Circle,
    response_count: usize,
    member_count: usize,
    metadata: String,
    anonymous_username: Option<String>,
    anonymous_avatar: Option<String>,
    // ...
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    is_blocked: bool,
    is_powerful: bool,
    is_member: bool,
    is_owner: bool,
}

/// GET /circles/@:name/inbox
pub async fn inbox_request(
    jar: CookieJar,
    Path(name): Path<String>,
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
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

    let mut is_owner = false;
    let is_member = if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Active
    } else {
        false
    };

    if !is_member {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    let questions = match database
        .get_questions_by_recipient(format!("circle:{}", circle.id))
        .await
    {
        Ok(unread) => unread,
        Err(_) => Vec::new(),
    };

    Html(
        InboxTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            circle: circle.clone(),
            questions,
            response_count: database
                .get_response_count_by_circle(circle.id.clone())
                .await,
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            anonymous_username: Some(
                circle
                    .metadata
                    .kv
                    .get("sparkler:anonymous_username")
                    .unwrap_or(&"anonymous".to_string())
                    .to_string(),
            ),
            anonymous_avatar: Some(
                circle
                    .metadata
                    .kv
                    .get("sparkler:anonymous_avatar")
                    .unwrap_or(&"/static/images/default-avatar.svg".to_string())
                    .to_string(),
            ),
            // ...
            layout: circle
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
            lock_profile: circle
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true",
            disallow_anonymous: circle
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true",
            require_account: circle
                .metadata
                .kv
                .get("sparkler:require_account")
                .unwrap_or(&"false".to_string())
                == "true",
            is_blocked: if let Some(block_list) = circle.metadata.kv.get("sparkler:block_list") {
                block_list.contains(&format!("<@{posting_as}>"))
            } else {
                false
            },
            is_powerful,
            is_member,
            is_owner,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "circle/settings/profile.html")]
struct ProfileSettingsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    circle: Circle,
    metadata: String,
}

/// GET /circles/@:name/settings
pub async fn profile_settings_request(
    jar: CookieJar,
    Path(name): Path<String>,
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let mut is_owner = false;
    if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Active
    } else {
        false
    };

    if !is_owner {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        ProfileSettingsTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            circle: circle.clone(),
            metadata: clean_metadata(&circle.metadata),
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "circle/settings/privacy.html")]
struct PrivacySettingsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    circle: Circle,
    metadata: String,
}

/// GET /circles/@:name/settings/privacy
pub async fn privacy_settings_request(
    jar: CookieJar,
    Path(name): Path<String>,
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let mut is_owner = false;
    if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Active
    } else {
        false
    };

    if !is_owner {
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        PrivacySettingsTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            circle: circle.clone(),
            metadata: clean_metadata(&circle.metadata),
        }
        .render()
        .unwrap(),
    )
}
