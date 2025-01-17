use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::extract::State;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use axum::Json;

use authbeam::model::{Permission, Profile, UserFollow, Warning};
use rainbeam::model::ResponseComment;

use databeam::DefaultReturn;
use crate::database::Database;
use crate::model::{Chat, DatabaseError, FullResponse, Question, RelationshipStatus};

use super::{clean_metadata, CleanProfileQuery, PaginatedQuery, ProfileQuery};

#[derive(Serialize, Deserialize)]
struct ProfileTemplate {
    other: Box<Profile>,
    response_count: usize,
    questions_count: usize,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    pinned: Option<Vec<FullResponse>>,
    page: i32,
    tag: String,
    query: String,
    // ...
    relationship: RelationshipStatus,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    layout: String,
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
    is_self: bool,
}

/// GET /@{username}
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

    let other = match database.auth.get_profile(username.clone()).await {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
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

    // ...
    let pinned = if let Some(pinned) = other.metadata.kv.get("sparkler:pinned") {
        if pinned.is_empty() {
            None
        } else {
            let mut out = Vec::new();

            for id in pinned.split(",") {
                match database.get_response(id.to_string()).await {
                    Ok(response) => {
                        if (response.1.author.id != other.id) | !pinned.contains(&response.1.id) {
                            // don't allow us to pin responses from other users
                            continue;
                        }

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

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    let relationship = if is_self | is_helper {
        // we're always friends with ourselves! (and staff)
        // allows user to bypass their own locked profile
        RelationshipStatus::Friends
    } else {
        if let Some(ref profile) = auth_user {
            database
                .auth
                .get_user_relationship(other.id.clone(), profile.id.clone())
                .await
                .0
        } else {
            RelationshipStatus::Unknown
        }
    };

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ProfileTemplate {
            other: other.clone(),
            response_count: database
                .get_response_count_by_author(other.id.clone())
                .await,
            questions_count: database
                .get_global_questions_count_by_author(other.id.clone())
                .await,
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
            pinned,
            page: query.page,
            tag: query.tag.unwrap_or(String::new()),
            query: query.q.unwrap_or(String::new()),
            // ...
            relationship,
            layout: other
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
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
            is_powerful,
            is_helper,
            is_self,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialProfileTemplate {
    other: Box<Profile>,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    // ...
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
}

/// GET /@{username}/_app/feed.html
pub async fn partial_profile_request(
    jar: CookieJar,
    Path(username): Path<String>,
    State(database): State<Database>,
    Query(query): Query<CleanProfileQuery>,
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

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let responses_original = if let Some(ref tag) = query.tag {
        // tagged
        match database
            .get_responses_by_author_tagged_paginated(
                other.id.to_owned(),
                tag.to_owned(),
                query.page,
            )
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Json(e.to_json()),
        }
    } else {
        if let Some(ref search) = query.q {
            // search
            match database
                .get_responses_by_author_searched_paginated(
                    other.id.to_owned(),
                    search.to_owned(),
                    query.page,
                )
                .await
            {
                Ok(responses) => responses,
                Err(e) => return Json(e.to_json()),
            }
        } else {
            // normal
            match database
                .get_responses_by_author_paginated(other.id.to_owned(), query.page)
                .await
            {
                Ok(responses) => responses,
                Err(e) => return Json(e.to_json()),
            }
        }
    };

    let mut responses = Vec::new();
    if query.clean {
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

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    let relationship = if is_self | is_helper {
        // we're always friends with ourselves! (and staff)
        // allows user to bypass their own locked profile
        RelationshipStatus::Friends
    } else {
        if let Some(ref profile) = auth_user {
            database
                .auth
                .get_user_relationship(other.id.clone(), profile.id.clone())
                .await
                .0
        } else {
            RelationshipStatus::Unknown
        }
    };

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

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
            other: other.clone(),
            responses,
            relationships,
            // ...
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ProfileEmbedTemplate {
    other: Box<Profile>,
    responses: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    pinned: Option<Vec<FullResponse>>,
    is_powerful: bool,
    is_helper: bool,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
}

/// GET /@{username}/embed
pub async fn profile_embed_request(
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
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
    };

    let mut responses = match database
        .get_responses_by_author_paginated(other.id.to_owned(), 0)
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let pinned = if let Some(pinned) = other.metadata.kv.get("sparkler:pinned") {
        if pinned.is_empty() {
            None
        } else {
            let mut out = Vec::new();

            for id in pinned.split(",") {
                match database.get_response(id.to_string()).await {
                    Ok(response) => {
                        if (response.1.author.id != other.id) | !pinned.contains(&response.1.id) {
                            // don't allow us to pin responses from other users
                            continue;
                        }

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

    // permissions
    let lock_profile = other
        .metadata
        .kv
        .get("sparkler:lock_profile")
        .unwrap_or(&"false".to_string())
        == "true";

    let disallow_anonymous = other
        .metadata
        .kv
        .get("sparkler:disallow_anonymous")
        .unwrap_or(&"false".to_string())
        == "true";

    let require_account = other
        .metadata
        .kv
        .get("sparkler:require_account")
        .unwrap_or(&"false".to_string())
        == "true";

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

    let relationship = if let Some(ref profile) = auth_user {
        database
            .auth
            .get_user_relationship(other.id.clone(), profile.id.clone())
            .await
            .0
    } else {
        RelationshipStatus::Unknown
    };

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

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
        payload: crate::routing::into_some_serde_value(ProfileEmbedTemplate {
            other: other.clone(),
            responses,
            relationships,
            pinned,
            is_powerful,
            is_helper,
            lock_profile,
            disallow_anonymous,
            require_account,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct FollowersTemplate {
    other: Box<Profile>,
    followers: Vec<(UserFollow, Box<Profile>, Box<Profile>)>,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    page: i32,
    // ...
    is_self: bool,
    is_helper: bool,
}

/// GET /@{username}/followers
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

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let is_helper = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
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
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let relationship = if is_self {
        // we're always friends with ourselves!
        // allows user to bypass their own locked profile
        RelationshipStatus::Friends
    } else {
        if let Some(ref profile) = auth_user {
            database
                .auth
                .get_user_relationship(other.id.clone(), profile.id.clone())
                .await
                .0
        } else {
            RelationshipStatus::Unknown
        }
    };

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(FollowersTemplate {
            other: other.clone(),
            followers: database
                .auth
                .get_followers_paginated(other.id.clone(), query.page)
                .await
                .unwrap(),
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            page: query.page,
            // ...
            is_self,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct FollowingTemplate {
    other: Box<Profile>,
    followers_count: usize,
    friends_count: usize,
    following: Vec<(UserFollow, Box<Profile>, Box<Profile>)>,
    following_count: usize,
    page: i32,
    // ...
    is_self: bool,
    is_helper: bool,
}

/// GET /@{username}/following
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

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let is_helper = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
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
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let relationship = if is_self {
        // we're always friends with ourselves!
        // allows user to bypass their own locked profile
        RelationshipStatus::Friends
    } else {
        if let Some(ref profile) = auth_user {
            database
                .auth
                .get_user_relationship(other.id.clone(), profile.id.clone())
                .await
                .0
        } else {
            RelationshipStatus::Unknown
        }
    };

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(FollowingTemplate {
            other: other.clone(),
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            following: database
                .auth
                .get_following_paginated(other.id.clone(), query.page)
                .await
                .unwrap(),
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            page: query.page,
            // ...
            is_self,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct FriendsTemplate {
    other: Box<Profile>,
    friends: Vec<(Box<Profile>, Box<Profile>)>,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    page: i32,
    // ...
    is_self: bool,
    is_helper: bool,
}

/// GET /@{username}/friends
pub async fn friends_request(
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

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let is_helper = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
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
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let relationship = if is_self {
        // we're always friends with ourselves!
        // allows user to bypass their own locked profile
        RelationshipStatus::Friends
    } else {
        if let Some(ref profile) = auth_user {
            database
                .auth
                .get_user_relationship(other.id.clone(), profile.id.clone())
                .await
                .0
        } else {
            RelationshipStatus::Unknown
        }
    };

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(FriendsTemplate {
            other: other.clone(),
            friends: database
                .auth
                .get_user_participating_relationships_of_status_paginated(
                    other.id.clone(),
                    RelationshipStatus::Friends,
                    query.page,
                )
                .await
                .unwrap(),
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            page: query.page,
            // ...
            is_self,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct FriendRequestsTemplate {
    other: Box<Profile>,
    requests: Vec<(Box<Profile>, Box<Profile>)>,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    page: i32,
    // ...
    is_self: bool,
    is_helper: bool,
}

/// GET /@{username}/friends/requests
pub async fn friend_requests_request(
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

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let is_self = auth_user.id == other.id;

    if !is_self && !is_helper {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(FriendRequestsTemplate {
            other: other.clone(),
            requests: database
                .auth
                .get_user_participating_relationships_of_status_paginated(
                    other.id.clone(),
                    RelationshipStatus::Pending,
                    query.page,
                )
                .await
                .unwrap(),
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            page: query.page,
            // ...
            is_self,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct BlocksTemplate {
    other: Box<Profile>,
    blocks: Vec<(Box<Profile>, Box<Profile>)>,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    page: i32,
    // ...
    is_self: bool,
    is_helper: bool,
}

/// GET /@{username}/friends/blocks
pub async fn blocks_request(
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

    if !is_helper {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let is_self = auth_user.id == other.id;

    if !is_self && !is_helper {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(BlocksTemplate {
            other: other.clone(),
            blocks: database
                .auth
                .get_user_participating_relationships_of_status_paginated(
                    other.id.clone(),
                    RelationshipStatus::Blocked,
                    query.page,
                )
                .await
                .unwrap(),
            followers_count: database.auth.get_followers_count(other.id.clone()).await,
            following_count: database.auth.get_following_count(other.id.clone()).await,
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            page: query.page,
            // ...
            is_self,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ProfileQuestionsTemplate {
    other: Box<Profile>,
    questions: Vec<(Question, usize, usize)>,
    questions_count: usize,
    response_count: usize,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    page: i32,
    query: String,
    // ...
    relationship: RelationshipStatus,
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    is_powerful: bool,
    is_helper: bool,
    is_self: bool,
}

/// GET /@{username}/questions
pub async fn questions_request(
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

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
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

    let questions = if let Some(ref search) = query.q {
        match database
            .get_global_questions_by_author_searched_paginated(
                other.id.to_owned(),
                search.clone(),
                query.page,
            )
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Json(e.to_json()),
        }
    } else {
        match database
            .get_global_questions_by_author_paginated(other.id.to_owned(), query.page)
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Json(e.to_json()),
        }
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

    let is_self = if let Some(ref profile) = auth_user {
        profile.id == other.id
    } else {
        false
    };

    let relationship = if is_self {
        // we're always friends with ourselves!
        // allows user to bypass their own locked profile
        RelationshipStatus::Friends
    } else {
        if let Some(ref profile) = auth_user {
            database
                .auth
                .get_user_relationship(other.id.clone(), profile.id.clone())
                .await
                .0
        } else {
            RelationshipStatus::Unknown
        }
    };

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ProfileQuestionsTemplate {
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
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
            page: query.page,
            query: query.q.unwrap_or(String::new()),
            // ...
            relationship,
            layout: other
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
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
            is_powerful,
            is_helper,
            is_self,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ModTemplate {
    other: Box<Profile>,
    warnings: Vec<Warning>,
    response_count: usize,
    questions_count: usize,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    badges: String,
    chats: Vec<(Chat, Vec<Box<Profile>>)>,
    tokens: String,
    tokens_src: Vec<String>,
    // ...
    relationship: RelationshipStatus,
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    is_powerful: bool,
    is_helper: bool,
    is_self: bool,
}

/// GET /@{username}/mod
pub async fn mod_request(
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let mut other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(_) => return Json(DatabaseError::NotFound.to_json()),
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

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        if group.permissions.contains(&Permission::Helper) {
            is_helper = true;
        }

        group.permissions.contains(&Permission::Manager)
    };

    if !is_helper {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    if other.group == -1 {
        other.group = -2;
    }

    let warnings = match database
        .auth
        .get_warnings_by_recipient(other.id.clone(), auth_user.clone())
        .await
    {
        Ok(r) => r,
        Err(_) => return Json(DatabaseError::Other.to_json()),
    };

    let is_self = auth_user.id == other.id;
    let relationship = RelationshipStatus::Friends; // moderators should always be your friend! (bypass private profile)

    let chats = match database.get_chats_for_user(other.id.clone()).await {
        Ok(c) => c,
        Err(e) => return Json(e.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ModTemplate {
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
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
            badges: serde_json::to_string_pretty(&other.badges).unwrap(),
            chats,
            tokens: serde_json::to_string(&other.tokens).unwrap(),
            tokens_src: other.tokens.clone(),
            // ...
            relationship,
            layout: other
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
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
            is_powerful,
            is_helper,
            is_self,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ProfileQuestionsInboxTemplate {
    other: Box<Profile>,
    questions: Vec<Question>,
    questions_count: usize,
    response_count: usize,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    // ...
    relationship: RelationshipStatus,
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    is_powerful: bool,
    is_helper: bool,
    is_self: bool,
}

/// GET /@{username}/questions/inbox
pub async fn inbox_request(
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
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

    let questions = match database
        .get_questions_by_recipient(other.id.to_owned())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        if group.permissions.contains(&Permission::Helper) {
            is_helper = true;
        }

        group.permissions.contains(&Permission::Manager)
    };

    if !is_powerful {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let is_self = auth_user.id == other.id;

    let relationship = database
        .auth
        .get_user_relationship(other.id.clone(), auth_user.id.clone())
        .await
        .0;

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ProfileQuestionsInboxTemplate {
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
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
            // ...
            relationship,
            layout: other
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
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
            is_powerful,
            is_helper,
            is_self,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ProfileQuestionsOutboxTemplate {
    other: Box<Profile>,
    questions: Vec<Question>,
    questions_count: usize,
    response_count: usize,
    followers_count: usize,
    following_count: usize,
    friends_count: usize,
    is_following: bool,
    is_following_you: bool,
    metadata: String,
    page: i32,
    // ...
    relationship: RelationshipStatus,
    layout: String,
    lock_profile: bool,
    disallow_anonymous: bool,
    require_account: bool,
    hide_social: bool,
    is_powerful: bool,
    is_helper: bool,
    is_self: bool,
}

/// GET /@{username}/questions/outbox
pub async fn outbox_request(
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
            Ok(ua) => ua,
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
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

    let questions = match database
        .get_questions_by_author_paginated(other.id.to_owned(), query.page)
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    let mut is_helper: bool = false;
    let is_powerful = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        if group.permissions.contains(&Permission::Helper) {
            is_helper = true;
        }

        group.permissions.contains(&Permission::Manager)
    };

    let is_self = auth_user.id == other.id;

    if !is_powerful && !is_self {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let relationship = database
        .auth
        .get_user_relationship(other.id.clone(), auth_user.id.clone())
        .await
        .0;

    let is_blocked = relationship == RelationshipStatus::Blocked;

    if !is_helper && is_blocked {
        return Json(DatabaseError::NotFound.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ProfileQuestionsOutboxTemplate {
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
            friends_count: database
                .auth
                .get_friendship_count_by_user(other.id.clone())
                .await,
            is_following,
            is_following_you,
            metadata: clean_metadata(&other.metadata),
            page: query.page,
            // ...
            relationship,
            layout: other
                .metadata
                .kv
                .get("sparkler:layout")
                .unwrap_or(&String::new())
                .to_owned(),
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
            is_powerful,
            is_helper,
            is_self,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct FriendRequestTemplate {
    other: Box<Profile>,
}

/// GET /@{username}/relationship/friend_accept
pub async fn friend_request(
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let relationship = database
        .auth
        .get_user_relationship(other.id.clone(), auth_user.id.clone())
        .await;

    // the relationship status must be pending AND we must be user 2 (the user who got sent the request)
    if (relationship.0 != RelationshipStatus::Pending) | (relationship.2 != auth_user.id) {
        return Json(DatabaseError::NotFound.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(FriendRequestTemplate {
            other: other.clone(),
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct WarningTemplate {
    other: Box<Profile>,
}

/// GET /@{username}/_app/warning
pub async fn warning_request(
    Path(username): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    let other = match database
        .auth
        .get_profile_by_username(username.clone())
        .await
    {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(WarningTemplate {
            other: other.clone(),
        }),
    })
}
