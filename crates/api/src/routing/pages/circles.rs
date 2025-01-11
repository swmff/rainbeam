use std::collections::HashMap;

use ammonia::Builder;

use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Redirect};
use axum::extract::State;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use axum::Json;

use authbeam::model::{Permission, Profile, RelationshipStatus};

use databeam::DefaultReturn;
use crate::database::Database;
use crate::model::{Circle, CircleMetadata, DatabaseError, FullResponse, MembershipStatus};

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

#[derive(Serialize, Deserialize)]
struct CirclesTemplate {
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(CirclesTemplate {
            circles: match database.get_user_circle_memberships(auth_user.id).await {
                Ok(c) => c,
                Err(e) => return Json(e.to_json()),
            },
        }),
    })
}

pub async fn profile_redirect_request(Path(name): Path<String>) -> impl IntoResponse {
    Redirect::to(&format!("/+{name}"))
}

#[derive(Serialize, Deserialize)]
struct ProfileTemplate {
    circle: Circle,
    responses: Vec<FullResponse>,
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

/// GET /+{name}
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let mut responses = match database
        .get_responses_by_circle_paginated(circle.id.to_owned(), query.page)
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
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
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let mut is_owner = false;
    let is_member = if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        let membership = database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await;

        (membership == MembershipStatus::Active) | (membership == MembershipStatus::Moderator)
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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ProfileTemplate {
            circle: circle.clone(),
            responses,
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
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialProfileTemplate {
    other: Circle,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    // ...
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
}

/// GET /+{name}/_app/feed.html
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
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let responses = match database
        .get_responses_by_circle_paginated(circle.id.to_owned(), query.page)
        .await
    {
        Ok(responses) => responses,
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
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PartialProfileTemplate {
            other: circle.clone(),
            responses,
            relationships,
            // ...
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct MemberlistTemplate {
    circle: Circle,
    members: Vec<Box<Profile>>,
    member_count: usize,
    metadata: String,
    // ...
    is_powerful: bool,
    is_member: bool,
    is_owner: bool,
}

/// GET /circles/@{name}/memberlist
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let members = match database.get_circle_memberships(circle.id.to_owned()).await {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let mut is_owner = false;
    let is_member = if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        let membership = database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await;

        (membership == MembershipStatus::Active) | (membership == MembershipStatus::Moderator)
    } else {
        false
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(MemberlistTemplate {
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
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct AcceptInviteTemplate {
    circle: Circle,
    member_count: usize,
    metadata: String,
    // ...
    is_powerful: bool,
    is_member: bool,
    is_owner: bool,
}

/// GET /circles/@{name}/memberlist/accept
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let mut is_owner = false;
    let is_member = if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        let membership = database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await;

        (membership == MembershipStatus::Active) | (membership == MembershipStatus::Moderator)
    } else {
        false
    };

    if is_member {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(AcceptInviteTemplate {
            circle: circle.clone(),
            member_count: database
                .get_circle_memberships_count(circle.id.clone())
                .await,
            metadata: clean_metadata(&circle.metadata),
            // ...
            is_powerful,
            is_member,
            is_owner,
        }),
    })
}
#[derive(Serialize, Deserialize)]
struct GeneralSettingsTemplate {
    circle: Circle,
    metadata: String,
}

/// GET /circles/@{name}/settings
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let mut is_owner = false;
    if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Moderator
    } else {
        false
    };

    if !is_owner {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(GeneralSettingsTemplate {
            circle: circle.clone(),
            metadata: clean_metadata(&circle.metadata),
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PrivacySettingsTemplate {
    circle: Circle,
    metadata: String,
}

/// GET /circles/@{name}/settings/privacy
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

    let circle = match database.get_circle_by_name(name.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let mut is_owner = false;
    if let Some(ref profile) = auth_user {
        is_owner = profile.id == circle.owner.id;

        database
            .get_user_circle_membership(profile.id.clone(), circle.id.clone())
            .await
            == MembershipStatus::Moderator
    } else {
        false
    };

    if !is_owner {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PrivacySettingsTemplate {
            circle: circle.clone(),
            metadata: clean_metadata(&circle.metadata),
        }),
    })
}
