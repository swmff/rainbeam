use axum::response::IntoResponse;
use axum::extract::{State, Query, Path};
use axum_extra::extract::CookieJar;
use axum::Json;

use authbeam::model::{Item, ItemStatus, ItemType, Permission, Profile};
use serde::{Deserialize, Serialize};

use databeam::DefaultReturn;
use crate::database::Database;
use crate::model::DatabaseError;

use super::MarketQuery;

#[derive(Serialize, Deserialize)]
struct HomepageTemplate {
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
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // permissions
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    if (props.status != ItemStatus::Approved) && (props.status != ItemStatus::Featured) {
        // check permission to see unapproved items
        if !group.permissions.contains(&Permission::Manager) {
            // we must have the "Manager" permission to edit other users
            return Json(DatabaseError::NotAllowed.to_json());
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
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        }
    } else {
        if (auth_user.id != props.creator) && !is_helper {
            // we cannot sort by somebody that isnt us if we arent helper
            return Json(DatabaseError::NotAllowed.to_json());
        }

        if let Some(r#type) = props.r#type {
            // creator and type
            match database
                .auth
                .get_items_by_creator_type_paginated(props.creator.clone(), r#type, props.page)
                .await
            {
                Ok(i) => i,
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    })
                }
            }
        } else {
            // no type, just creator
            match database
                .auth
                .get_items_by_creator_paginated(props.creator.clone(), props.page)
                .await
            {
                Ok(i) => i,
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    })
                }
            }
        }
    };

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(HomepageTemplate {
            page: props.page,
            query: props.q,
            status: props.status,
            creator: props.creator,
            items,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ItemTemplate {
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
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(_) => return Json(DatabaseError::NotAllowed.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // permissions
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let is_helper = group.permissions.contains(&Permission::Helper);

    // data
    let item = match database.auth.get_item(id.clone()).await {
        Ok(i) => i,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    if !is_helper
        && (item.status != ItemStatus::Approved)
        && (item.status != ItemStatus::Featured)
        && auth_user.id != item.creator
    {
        // users who aren't helpers cannot view unapproved items
        return Json(DatabaseError::NotAllowed.to_json());
    }

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ItemTemplate {
            is_owned: database
                .auth
                .get_transaction_by_customer_item(auth_user.id.clone(), item.id.clone())
                .await
                .is_ok(),
            creator: match database.auth.get_profile(item.creator.clone()).await {
                Ok(ua) => ua,
                Err(e) => {
                    return Json(DefaultReturn {
                        success: false,
                        message: e.to_string(),
                        payload: None,
                    })
                }
            },
            item,
            is_helper,
            reaction_count: database.get_reaction_count_by_asset(id).await,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ThemePlaygroundTemplate {
    css: String,
}

/// GET /market/_app/theme_playground.html
pub async fn theme_playground_request(
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // data
    let item = match database.auth.get_item(id.clone()).await {
        Ok(i) => i,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    if item.r#type != ItemType::UserTheme {
        return Json(DatabaseError::ValueError.to_json());
    }

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ThemePlaygroundTemplate {
            css: item.content,
        }),
    })
}
