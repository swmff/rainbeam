use askama_axum::Template;
use axum::response::IntoResponse;
use axum::{
    extract::{State, Query, Path},
    response::Html,
};
use axum_extra::extract::CookieJar;

use authbeam::model::{Item, ItemStatus, ItemType, Permission, Profile};

use crate::config::Config;
use crate::database::Database;
use crate::model::DatabaseError;
use crate::ToHtml;

use super::MarketQuery;

#[derive(Template)]
#[template(path = "market/homepage.html")]
struct HomepageTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    page: i32,
    query: String,
    status: ItemStatus,
    creator: String,
    items: Vec<(Item, Box<Profile>)>,
    is_helper: bool,
}

/// GET /market
pub async fn homepage_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<MarketQuery>,
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

    // permissions
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => return Html(e.to_string()),
    };

    if (props.status != ItemStatus::Approved) && (props.status != ItemStatus::Featured) {
        // check permission to see unapproved items
        if !group.permissions.contains(&Permission::Manager) {
            // we must have the "Manager" permission to edit other users
            return Html(DatabaseError::NotAllowed.to_string());
        }
    }

    let is_helper = group.permissions.contains(&Permission::Helper);

    // data
    let items = if props.creator.is_empty() {
        match database
            .auth
            .get_items_by_status_searched_paginated(
                props.status.clone(),
                props.page,
                props.q.clone(),
            )
            .await
        {
            Ok(i) => i,
            Err(e) => return Html(e.to_string()),
        }
    } else {
        if let Some(r#type) = props.r#type {
            // creator and type
            match database
                .auth
                .get_items_by_creator_type_paginated(props.creator.clone(), r#type, props.page)
                .await
            {
                Ok(i) => i,
                Err(e) => return Html(e.to_string()),
            }
        } else {
            // no type, just creator
            match database
                .auth
                .get_items_by_creator_paginated(props.creator.clone(), props.page)
                .await
            {
                Ok(i) => i,
                Err(e) => return Html(e.to_string()),
            }
        }
    };

    // ...
    Html(
        HomepageTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            notifs,
            page: props.page,
            query: props.q,
            status: props.status,
            creator: props.creator,
            items,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "market/new.html")]
struct CreateTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
}

/// GET /market/new
pub async fn create_request(jar: CookieJar, State(database): State<Database>) -> impl IntoResponse {
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
        CreateTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "market/item.html")]
struct ItemTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
    item: Item,
    creator: Box<Profile>,
    is_owned: bool,
    is_helper: bool,
    reaction_count: usize,
}

/// GET /market/item/:id
pub async fn item_request(
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

    // permissions
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => return Html(e.to_string()),
    };

    let is_helper = group.permissions.contains(&Permission::Helper);

    // data
    let item = match database.auth.get_item(id.clone()).await {
        Ok(i) => i,
        Err(e) => return Html(e.to_string()),
    };

    if !is_helper
        && (item.status != ItemStatus::Approved)
        && (item.status != ItemStatus::Featured)
        && auth_user.id != item.creator
    {
        // users who aren't helpers cannot view unapproved items
        return Html(DatabaseError::NotAllowed.to_string());
    }

    // ...
    Html(
        ItemTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            is_owned: database
                .auth
                .get_transaction_by_customer_item(auth_user.id.clone(), item.id.clone())
                .await
                .is_ok(),
            profile: Some(auth_user),
            unread,
            notifs,
            creator: match database.auth.get_profile(item.creator.clone()).await {
                Ok(ua) => ua,
                Err(e) => return Html(e.to_string()),
            },
            item,
            is_helper,
            reaction_count: database.get_reaction_count_by_asset(id).await,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "partials/components/theme_playground.html")]
struct ThemePlaygroundTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    css: String,
}

/// GET /market/_app/theme_playground.html
pub async fn theme_playground_request(
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
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    // data
    let item = match database.auth.get_item(id.clone()).await {
        Ok(i) => i,
        Err(e) => return Html(e.to_string()),
    };

    if item.r#type != ItemType::UserTheme {
        return Html(DatabaseError::ValueError.to_string());
    }

    // ...
    Html(
        ThemePlaygroundTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            css: item.content,
        }
        .render()
        .unwrap(),
    )
}
