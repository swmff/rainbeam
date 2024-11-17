use std::collections::HashMap;

use ammonia::Builder;
use askama_axum::Template;
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Redirect};
use axum::{extract::State, response::Html};
use axum_extra::extract::CookieJar;

use authbeam::model::{Permission, Profile, RelationshipStatus};

use crate::config::Config;
use crate::database::Database;
use crate::model::{Circle, CircleMetadata, DatabaseError, FullResponse, MembershipStatus};
use crate::ToHtml;

use super::{PaginatedQuery, ProfileQuery};

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
                .replace("url(\"", "url(\"/api/v0/util/ext/image?img=")
                .replace("url(https://", "url(/api/v0/util/ext/image?img=https://")
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
    notifs: usize,
    circle: Circle,
    responses: Vec<FullResponse>,
    reactions: Vec<String>,
    relationships: HashMap<String, RelationshipStatus>,
    member_count: usize,
    metadata: String,
    pinned: Option<Vec<FullResponse>>,
    page: i32,
    // ...
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
                match database.get_response(id.to_string()).await {
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

    // collect all responses we've reacted to
    let mut reactions: Vec<String> = Vec::new();

    if let Some(ref ua) = auth_user {
        for response in &responses {
            if let Ok(_) = database
                .get_reaction(ua.id.clone(), response.1.id.clone())
                .await
            {
                reactions.push(response.1.id.clone())
            }
        }
    }

    // ...
    Html(
        ProfileTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            unread,
            notifs,
            circle: circle.clone(),
            responses,
            reactions,
            relationships,
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            pinned,
            page: query.page,
            // ...
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
#[template(path = "partials/profile/feed.html")]
struct PartialProfileTemplate {
    config: Config,
    profile: Option<Profile>,
    other: Circle,
    responses: Vec<FullResponse>,
    reactions: Vec<String>,
    relationships: HashMap<String, RelationshipStatus>,
    // ...
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
}

/// GET /+:name/_app/feed.html
pub async fn partial_profile_request(
    jar: CookieJar,
    Path(name): Path<String>,
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Html(DatabaseError::NotFound.to_html(database)),
    };

    let responses = match database
        .get_responses_by_circle_paginated(circle.id.to_owned(), query.page)
        .await
    {
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

    // collect all responses we've reacted to
    let mut reactions: Vec<String> = Vec::new();

    if let Some(ref ua) = auth_user {
        for response in &responses {
            if let Ok(_) = database
                .get_reaction(ua.id.clone(), response.1.id.clone())
                .await
            {
                reactions.push(response.1.id.clone())
            }
        }
    }

    // ...
    Html(
        PartialProfileTemplate {
            config: database.server_options.clone(),
            profile: auth_user.clone(),
            other: circle.clone(),
            responses,
            reactions,
            relationships,
            // ...
            is_powerful,
            is_helper,
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
    notifs: usize,
    circle: Circle,
    members: Vec<Profile>,
    member_count: usize,
    metadata: String,
    // ...
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
            circle: circle.clone(),
            members,
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            // ...
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
    notifs: usize,
    circle: Circle,
    member_count: usize,
    metadata: String,
    // ...
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

    let notifs = if let Some(ref ua) = auth_user {
        database
            .auth
            .get_notification_count_by_recipient(ua.id.to_owned())
            .await
    } else {
        0
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
            circle: circle.clone(),
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            // ...
            is_powerful,
            is_member,
            is_owner,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "circle/settings/general.html")]
struct GeneralSettingsTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    circle: Circle,
    metadata: String,
}

/// GET /circles/@:name/settings
pub async fn general_settings_request(
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
        GeneralSettingsTemplate {
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
