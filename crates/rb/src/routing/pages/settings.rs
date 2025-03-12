use reva_axum::Template;
use axum::response::IntoResponse;
use axum::{
    extract::{State, Query},
    response::Html,
};
use axum_extra::extract::CookieJar;

use authbeam::model::{IpBlock, Item, Profile, Transaction};

use crate::config::Config;
use crate::database::Database;
use crate::model::{DatabaseError, RelationshipStatus};
use crate::ToHtml;

use super::{clean_metadata_short, NotificationsQuery};

#[derive(Template)]
#[template(path = "settings/account.html")]
struct AccountSettingsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = database.get_inbox_count_by_recipient(&auth_user.id).await;

    let notifs = database
        .auth
        .get_notification_count_by_recipient(&auth_user.id)
        .await;

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Html(e.to_html(database)),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the settings of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    let relationships = match database
        .auth
        .get_user_relationships_of_status(&user.id, RelationshipStatus::Blocked)
        .await
    {
        Ok(r) => r,
        Err(_) => Vec::new(),
    };

    let ipblocks = match database.auth.get_ipblocks(&user.id).await {
        Ok(r) => r,
        Err(_) => Vec::new(),
    };

    Html(
        AccountSettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
            relationships,
            ipblocks,
            user,
            viewing_other_profile,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "settings/profile.html")]
struct ProfileSettingsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = database.get_inbox_count_by_recipient(&auth_user.id).await;

    let notifs = database
        .auth
        .get_notification_count_by_recipient(&auth_user.id)
        .await;

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Html(e.to_html(database)),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the settings of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        ProfileSettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
            user,
            viewing_other_profile,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "settings/theme.html")]
struct ThemeSettingsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    metadata: String,
    user: Box<Profile>,
    viewing_other_profile: bool,
}

/// GET /settings/theme
pub async fn theme_settings(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<NotificationsQuery>,
) -> impl IntoResponse {
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .auth
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = database.get_inbox_count_by_recipient(&auth_user.id).await;

    let notifs = database
        .auth
        .get_notification_count_by_recipient(&auth_user.id)
        .await;

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Html(e.to_html(database)),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the settings of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        ThemeSettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
            user,
            viewing_other_profile,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "settings/privacy.html")]
struct PrivacySettingsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = database.get_inbox_count_by_recipient(&auth_user.id).await;

    let notifs = database
        .auth
        .get_notification_count_by_recipient(&auth_user.id)
        .await;

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Html(e.to_html(database)),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the settings of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        PrivacySettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
            user,
            viewing_other_profile,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "settings/sessions.html")]
struct SessionsSettingsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = database.get_inbox_count_by_recipient(&auth_user.id).await;

    let notifs = database
        .auth
        .get_notification_count_by_recipient(&auth_user.id)
        .await;

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Html(e.to_html(database)),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the settings of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    Html(
        SessionsSettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&user.metadata),
            tokens: serde_json::to_string(&user.tokens).unwrap(),
            tokens_src: user.tokens.clone(),
            profile: Some(auth_user),
            unread,
            notifs,
            user,
            current_session: rainbeam_shared::hash::hash(
                jar.get("__Secure-Token")
                    .unwrap()
                    .value_trimmed()
                    .to_string(),
            ),
            viewing_other_profile,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "settings/coins.html")]
struct CoinsSettingsTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let unread = database.get_inbox_count_by_recipient(&auth_user.id).await;

    let notifs = database
        .auth
        .get_notification_count_by_recipient(&auth_user.id)
        .await;

    let user = if props.profile.is_empty() {
        auth_user.clone()
    } else {
        match database.get_profile(props.profile.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Html(e.to_html(database)),
        }
    };

    let viewing_other_profile =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    if viewing_other_profile && !is_helper {
        // we cannot view the settings of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    let transactions = match database
        .auth
        .get_participating_transactions_paginated(&user.id, props.page)
        .await
    {
        Ok(t) => t,
        Err(e) => return Html(e.to_string()),
    };

    Html(
        CoinsSettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
            user,
            transactions,
            page: props.page,
            viewing_other_profile,
        }
        .render()
        .unwrap(),
    )
}
