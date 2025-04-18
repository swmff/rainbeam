use crate::database::Database;
use crate::model::{
    DatabaseError, ItemCreate, ItemEdit, ItemEditContent, ItemType, SetItemStatus, TokenPermission,
    TransactionCreate,
};
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

/// Get an item
pub async fn get_request(
    State(database): State<Database>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // get item
    let item = match database.get_item(&id).await {
        Ok(i) => i,
        Err(e) => return Json(e.to_json()),
    };

    // we can only view the content of modules through the api :)
    if item.r#type != ItemType::Module {
        return Json(DatabaseError::NotAllowed.to_json());
    }

    // return
    Json(DefaultReturn {
        success: true,
        message: item.id.to_string(),
        payload: Some(item),
    })
}

/// Create an item
pub async fn create_request(
    jar: CookieJar,
    State(database): State<Database>,
    Json(props): Json<ItemCreate>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => {
            let token = c.value_trimmed();
            match database.get_profile_by_unhashed(token).await {
                Ok(ua) => {
                    // check token permission
                    if !ua
                        .token_context_from_token(&token)
                        .can_do(TokenPermission::ManageAssets)
                    {
                        return Json(DatabaseError::NotAllowed.to_json());
                    }

                    // return
                    ua
                }
                Err(e) => return Json(e.to_json()),
            }
        }
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    let item = match database.create_item(props, &auth_user.id).await {
        Ok(m) => m,
        Err(e) => return Json(e.to_json()),
    };

    Json(DefaultReturn {
        success: true,
        message: "Item created".to_string(),
        payload: Some(item),
    })
}

/// Delete an item
pub async fn delete_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database.get_profile_by_unhashed(c.value_trimmed()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database.delete_item(&id, auth_user).await {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Item deleted".to_string(),
        payload: (),
    })
}

/// Update item status
pub async fn update_status_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<SetItemStatus>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database.get_profile_by_unhashed(c.value_trimmed()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database
        .update_item_status(&id, props.status, auth_user)
        .await
    {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Item updated".to_string(),
        payload: (),
    })
}

/// Update item fields
pub async fn update_item_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<ItemEdit>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database.get_profile_by_unhashed(c.value_trimmed()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database.update_item(&id, props, auth_user).await {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Item updated".to_string(),
        payload: (),
    })
}

/// Update item content
pub async fn update_item_content_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
    Json(props): Json<ItemEditContent>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database.get_profile_by_unhashed(c.value_trimmed()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    if let Err(e) = database.update_item_content(&id, props, auth_user).await {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Item updated".to_string(),
        payload: (),
    })
}

/// Buy an item
pub async fn buy_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database.get_profile_by_unhashed(c.value_trimmed()).await {
            Ok(ua) => ua,
            Err(e) => return Json(e.to_json()),
        },
        None => return Json(DatabaseError::NotAllowed.to_json()),
    };

    // return
    let item = match database.get_item(&id).await {
        Ok(i) => i,
        Err(e) => return Json(e.to_json()),
    };

    if item.cost == -1 {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::TooExpensive.to_string(),
            payload: (),
        });
    }

    // make sure we don't already have this item
    if let Ok(_) = database
        .get_transaction_by_customer_item(&auth_user.id, &item.id)
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::MustBeUnique.to_string(),
            payload: (),
        });
    }

    // ...
    if let Err(e) = database
        .create_transaction(
            TransactionCreate {
                merchant: item.creator.clone(),
                item: item.id.clone(),
                amount: -(item.cost),
            },
            &auth_user.id,
        )
        .await
    {
        return Json(e.to_json());
    }

    Json(DefaultReturn {
        success: true,
        message: "Purchase successful".to_string(),
        payload: (),
    })
}
