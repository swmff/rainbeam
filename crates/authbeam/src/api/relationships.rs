use crate::database::Database;
use crate::model::{DatabaseError, UserFollow, RelationshipStatus};
use databeam::prelude::DefaultReturn;

use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

/// Toggle following on the given user
pub async fn follow_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: (),
                });
            }
        },
        None => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotAllowed.to_string(),
                payload: (),
            });
        }
    };

    // check block status
    let attempting_to_follow = match database.get_profile(id.to_owned()).await {
        Ok(ua) => ua,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotFound.to_string(),
                payload: (),
            })
        }
    };

    let relationship = database
        .get_user_relationship(attempting_to_follow.id.clone(), auth_user.id.clone())
        .await
        .0;

    if relationship == RelationshipStatus::Blocked {
        // blocked users cannot follow the people who blocked them!
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: (),
        });
    }

    // return
    match database
        .toggle_user_follow(&mut UserFollow {
            user: auth_user.id,
            following: attempting_to_follow.id,
        })
        .await
    {
        Ok(_) => Json(DefaultReturn {
            success: true,
            message: "Follow toggled".to_string(),
            payload: (),
        }),
        Err(e) => Json(e.to_json()),
    }
}

/// Send/accept a friend request to/from another user
pub async fn friend_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                });
            }
        },
        None => {
            return Json(DatabaseError::NotAllowed.to_json());
        }
    };

    // ...
    let other_user = match database.get_profile(id.to_owned()).await {
        Ok(ua) => ua,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotFound.to_string(),
                payload: None,
            })
        }
    };

    // get current relationship
    let current = database
        .get_user_relationship(auth_user.id.clone(), other_user.id.clone())
        .await;

    if current.0 == RelationshipStatus::Blocked && auth_user.id != current.1 {
        // cannot change relationship if we're blocked and we aren't the user that did the blocking
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    let current = current.0;

    // return
    if current == RelationshipStatus::Unknown {
        // send request
        match database
            .set_user_relationship_status(
                auth_user.id,
                other_user.id,
                RelationshipStatus::Pending,
                false,
            )
            .await
        {
            Ok(export) => {
                return Json(DefaultReturn {
                    success: true,
                    message: "Friend request sent!".to_string(),
                    payload: Some(export),
                })
            }
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        }
    } else if current == RelationshipStatus::Pending {
        // accept request
        match database
            .set_user_relationship_status(
                auth_user.id,
                other_user.id,
                RelationshipStatus::Friends,
                false,
            )
            .await
        {
            Ok(export) => {
                return Json(DefaultReturn {
                    success: true,
                    message: "Friend request accepted!".to_string(),
                    payload: Some(export),
                })
            }
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        }
    } else {
        // no clue, remove friendship?
        match database
            .set_user_relationship_status(
                auth_user.id,
                other_user.id,
                RelationshipStatus::Unknown,
                false,
            )
            .await
        {
            Ok(export) => {
                return Json(DefaultReturn {
                    success: true,
                    message: "Friendship removed".to_string(),
                    payload: Some(export),
                })
            }
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                })
            }
        }
    }
}

/// Block another user
pub async fn block_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                });
            }
        },
        None => {
            return Json(DatabaseError::NotAllowed.to_json());
        }
    };

    // ...
    let other_user = match database.get_profile(id.to_owned()).await {
        Ok(ua) => ua,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotFound.to_string(),
                payload: None,
            })
        }
    };

    // get current relationship
    let current = database
        .get_user_relationship(auth_user.id.clone(), other_user.id.clone())
        .await;

    if current.0 == RelationshipStatus::Blocked && auth_user.id != current.1 {
        // cannot change relationship if we're blocked and we aren't the user that did the blocking
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // force unfollow
    if let Err(e) = database
        .force_remove_user_follow(&mut UserFollow {
            user: auth_user.id.clone(),
            following: other_user.id.clone(),
        })
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    if let Err(e) = database
        .force_remove_user_follow(&mut UserFollow {
            user: other_user.id.clone(),
            following: auth_user.id.clone(),
        })
        .await
    {
        return Json(DefaultReturn {
            success: false,
            message: e.to_string(),
            payload: None,
        });
    }

    // return
    match database
        .set_user_relationship_status(
            auth_user.id,
            other_user.id,
            RelationshipStatus::Blocked,
            false,
        )
        .await
    {
        Ok(export) => {
            return Json(DefaultReturn {
                success: true,
                message: "User blocked!".to_string(),
                payload: Some(export),
            })
        }
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    }
}

/// Remove relationship with another user
pub async fn delete_request(
    jar: CookieJar,
    Path(id): Path<String>,
    State(database): State<Database>,
) -> impl IntoResponse {
    // get user from token
    let auth_user = match jar.get("__Secure-Token") {
        Some(c) => match database
            .get_profile_by_unhashed(c.value_trimmed().to_string())
            .await
        {
            Ok(ua) => ua,
            Err(e) => {
                return Json(DefaultReturn {
                    success: false,
                    message: e.to_string(),
                    payload: None,
                });
            }
        },
        None => {
            return Json(DatabaseError::NotAllowed.to_json());
        }
    };

    // ...
    let other_user = match database.get_profile(id.to_owned()).await {
        Ok(ua) => ua,
        Err(_) => {
            return Json(DefaultReturn {
                success: false,
                message: DatabaseError::NotFound.to_string(),
                payload: None,
            })
        }
    };

    // get current relationship
    let current = database
        .get_user_relationship(auth_user.id.clone(), other_user.id.clone())
        .await;

    if current.0 == RelationshipStatus::Blocked && auth_user.id != current.1 {
        // cannot remove relationship if we're blocked and we aren't the user that did the blocking
        return Json(DefaultReturn {
            success: false,
            message: DatabaseError::NotAllowed.to_string(),
            payload: None,
        });
    }

    // return
    match database
        .set_user_relationship_status(
            auth_user.id,
            other_user.id,
            RelationshipStatus::Unknown,
            false,
        )
        .await
    {
        Ok(export) => {
            return Json(DefaultReturn {
                success: true,
                message: "Relationship removed!".to_string(),
                payload: Some(export),
            })
        }
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    }
}
