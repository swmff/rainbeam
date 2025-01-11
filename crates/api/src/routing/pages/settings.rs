use axum::response::IntoResponse;
use axum::extract::{State, Query};
use axum::Json;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use authbeam::model::{IpBlock, Item, Permission, Profile, Transaction};

use databeam::DefaultReturn;
use crate::database::Database;
use crate::model::{DatabaseError, RelationshipStatus};

use super::{clean_metadata_short, NotificationsQuery};

#[derive(Serialize, Deserialize)]
struct AccountSettingsTemplate {
    metadata: String,
    relationships: Vec<(Box<Profile>, RelationshipStatus)>,
    ipblocks: Vec<IpBlock>,
    user: Box<Profile>,
    viewing_other_profile: bool,
}

/// GET /settings
pub async fn account_settings(
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

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let relationships = match database
        .auth
        .get_user_relationships_of_status(user.id.clone(), RelationshipStatus::Blocked)
        .await
    {
        Ok(r) => r,
        Err(_) => Vec::new(),
    };

    let ipblocks = match database.auth.get_ipblocks(user.id.clone()).await {
        Ok(r) => r,
        Err(_) => Vec::new(),
    };

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(AccountSettingsTemplate {
            metadata: clean_metadata_short(&user.metadata),
            relationships,
            ipblocks,
            user,
            viewing_other_profile,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ProfileSettingsTemplate {
    metadata: String,
    user: Box<Profile>,
    viewing_other_profile: bool,
}

/// GET /settings/profile
pub async fn profile_settings(
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

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ProfileSettingsTemplate {
            metadata: clean_metadata_short(&user.metadata),
            user,
            viewing_other_profile,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PrivacySettingsTemplate {
    metadata: String,
    user: Box<Profile>,
    viewing_other_profile: bool,
}

/// GET /settings/privacy
pub async fn privacy_settings(
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

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PrivacySettingsTemplate {
            metadata: clean_metadata_short(&user.metadata),
            user,
            viewing_other_profile,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct SessionsSettingsTemplate {
    metadata: String,
    tokens: String,
    tokens_src: Vec<String>,
    current_session: String,
    user: Box<Profile>,
    viewing_other_profile: bool,
}

/// GET /settings/sessions
pub async fn sessions_settings(
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

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Json(DatabaseError::NotAllowed.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(SessionsSettingsTemplate {
            metadata: clean_metadata_short(&user.metadata),
            tokens: serde_json::to_string(&user.tokens).unwrap(),
            tokens_src: user.tokens.clone(),
            user,
            current_session: rainbeam_shared::hash::hash(
                jar.get("__Secure-Token")
                    .unwrap()
                    .value_trimmed()
                    .to_string(),
            ),
            viewing_other_profile,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct CoinsSettingsTemplate {
    metadata: String,
    transactions: Vec<((Transaction, Option<Item>), Box<Profile>, Box<Profile>)>,
    page: i32,
    user: Box<Profile>,
    viewing_other_profile: bool,
}

/// GET /settings/coins
pub async fn coins_settings(
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

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let transactions = match database
        .auth
        .get_participating_transactions_paginated(user.id.clone(), props.page)
        .await
    {
        Ok(t) => t,
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
        payload: crate::routing::into_some_serde_value(CoinsSettingsTemplate {
            metadata: clean_metadata_short(&user.metadata),
            user,
            transactions,
            page: props.page,
            viewing_other_profile,
        }),
    })
}
