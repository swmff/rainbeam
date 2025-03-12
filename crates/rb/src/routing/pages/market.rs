use authbeam::layout::LayoutComponent;
use reva_axum::Template;
use axum::response::IntoResponse;
use axum::{
    extract::{State, Query, Path},
    response::Html,
};
use axum_extra::extract::CookieJar;

use authbeam::model::{FinePermission, Item, ItemStatus, ItemType, Profile};

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
    customer: String,
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

    // permissions
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => return Html(e.to_string()),
    };

    if (props.status != ItemStatus::Approved) && (props.status != ItemStatus::Featured) {
        // check permission to see unapproved items
        if !group.permissions.check(FinePermission::ECON_MASTER) {
            // we must have the "Manager" permission to edit other users
            return Html(DatabaseError::NotAllowed.to_string());
        }
    }

    let is_helper = group.permissions.check_helper();

    if !props.customer.is_empty() && (props.customer != auth_user.id) && !is_helper {
        // cannot view the owned items of anybody else (unless you're a helper)
        return Html(DatabaseError::NotAllowed.to_html(database));
    }

    // data
    let items = if props.creator.is_empty() {
        if let Some(r#type) = props.r#type {
            match database
                .auth
                .get_items_by_type_paginated(r#type, props.page)
                .await
            {
                Ok(i) => i,
                Err(e) => return Html(e.to_string()),
            }
        } else {
            match database
                .auth
                .get_items_by_status_searched_paginated(props.status.clone(), props.page, &props.q)
                .await
            {
                Ok(i) => i,
                Err(e) => return Html(e.to_string()),
            }
        }
    } else if !props.customer.is_empty() {
        match database
            .auth
            .get_transactions_by_customer_paginated(&props.customer, props.page)
            .await
        {
            Ok(i) => {
                let mut out = Vec::new();

                for x in i {
                    out.push((
                        match x.0 .1 {
                            Some(i) => i.clone(),
                            None => return Html(DatabaseError::NotFound.to_html(database)),
                        },
                        x.2.clone(),
                    ))
                }

                out
            }
            Err(e) => return Html(e.to_string()),
        }
    } else {
        if (auth_user.id != props.creator) && !is_helper {
            // we cannot sort by somebody that isnt us if we arent helper
            return Html(DatabaseError::NotAllowed.to_html(database));
        }

        if let Some(r#type) = props.r#type {
            // creator and type
            match database
                .auth
                .get_items_by_creator_type_paginated(&props.creator, r#type, props.page)
                .await
            {
                Ok(i) => i,
                Err(e) => return Html(e.to_string()),
            }
        } else {
            // no type, just creator
            match database
                .auth
                .get_items_by_creator_paginated(&props.creator, props.page)
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
            customer: props.customer,
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

/// GET /market/item/{id}
pub async fn item_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
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

    // permissions
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => return Html(e.to_string()),
    };

    let is_helper = group.permissions.check_helper();

    // data
    let item = match database.auth.get_item(&id).await {
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
                .get_transaction_by_customer_item(&auth_user.id, &item.id)
                .await
                .is_ok(),
            profile: Some(auth_user),
            unread,
            notifs,
            creator: match database.auth.get_profile(&item.creator).await {
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
            .get_profile_by_unhashed(c.value_trimmed())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    // data
    let item = match database.auth.get_item(&id).await {
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

#[derive(Template)]
#[template(path = "partials/components/layout_playground.html")]
struct LayoutPlaygroundTemplate {
    config: Config,
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    layout: LayoutComponent,
}

/// GET /market/_app/layout_playground.html
pub async fn layout_playground_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
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

    // data
    let item = match database.auth.get_item(&id).await {
        Ok(i) => i,
        Err(e) => return Html(e.to_string()),
    };

    if item.r#type != ItemType::Layout {
        return Html(DatabaseError::ValueError.to_string());
    }

    // ...
    Html(
        LayoutPlaygroundTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user),
            layout: match serde_json::from_str(&item.content) {
                Ok(l) => l,
                Err(_) => return Html(DatabaseError::ValueError.to_string()),
            },
        }
        .render()
        .unwrap(),
    )
}
