use askama_axum::Template;
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

use super::{clean_metadata_short, PaginatedQuery};

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
}

/// GET /settings
pub async fn account_settings(
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

    let relationships = match database
        .auth
        .get_user_relationships_of_status(auth_user.id.clone(), RelationshipStatus::Blocked)
        .await
    {
        Ok(r) => r,
        Err(_) => Vec::new(),
    };

    let ipblocks = match database.auth.get_ipblocks(auth_user.id.clone()).await {
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
            metadata: clean_metadata_short(&auth_user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
            relationships,
            ipblocks,
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
}

/// GET /settings/profile
pub async fn profile_settings(
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
        ProfileSettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&auth_user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
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
}

/// GET /settings/privacy
pub async fn privacy_settings(
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
        PrivacySettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&auth_user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
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
}

/// GET /settings/sessions
pub async fn sessions_settings(
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
        SessionsSettingsTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            metadata: clean_metadata_short(&auth_user.metadata),
            tokens: serde_json::to_string(&auth_user.tokens).unwrap(),
            tokens_src: auth_user.tokens.clone(),
            profile: Some(auth_user),
            unread,
            notifs,
            current_session: rainbeam_shared::hash::hash(
                jar.get("__Secure-Token")
                    .unwrap()
                    .value_trimmed()
                    .to_string(),
            ),
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
}

/// GET /settings/coins
pub async fn coins_settings(
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

    let transactions = match database
        .auth
        .get_participating_transactions_paginated(auth_user.id.clone(), props.page)
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
            metadata: clean_metadata_short(&auth_user.metadata),
            profile: Some(auth_user),
            unread,
            notifs,
            transactions,
            page: props.page,
        }
        .render()
        .unwrap(),
    )
}
