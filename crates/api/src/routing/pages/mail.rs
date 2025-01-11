use axum::extract::Path;
use axum::response::IntoResponse;
use axum::extract::{Query, State};
use axum_extra::extract::CookieJar;
use axum::Json;

use authbeam::model::{DatabaseError as AuthError, Mail, Permission, Profile};
use serde::{Deserialize, Serialize};

use databeam::DefaultReturn;
use crate::database::Database;
use crate::model::DatabaseError;
use super::NotificationsQuery;

#[derive(Serialize, Deserialize)]
struct InboxTemplate {
    mail: Vec<(Mail, Box<Profile>)>,
    page: i32,
    violating_usc18_1702: bool, // if we're trying to view the mail of another user
    pid: String,
    is_helper: bool,
}

/// GET /inbox/mail
pub async fn inbox_request(
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

    let violating_usc18_1702 =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if violating_usc18_1702 && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let mail = match database
        .auth
        .get_mail_by_recipient_paginated(
            if violating_usc18_1702 {
                props.profile.clone()
            } else {
                auth_user.id.clone()
            },
            props.page,
        )
        .await
    {
        Ok(c) => c,
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
        payload: crate::routing::into_some_serde_value(InboxTemplate {
            mail,
            page: props.page,
            violating_usc18_1702,
            pid: if violating_usc18_1702 {
                props.profile.clone()
            } else {
                auth_user.id.clone()
            },
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct OutboxTemplate {
    mail: Vec<(Mail, Box<Profile>)>,
    page: i32,
    violating_usc18_1702: bool,
    pid: String,
    is_helper: bool,
}

/// GET /inbox/mail/sent
pub async fn outbox_request(
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

    let violating_usc18_1702 =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if violating_usc18_1702 && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Json(DatabaseError::NotAllowed.to_json());
    }

    let mail = match database
        .auth
        .get_mail_by_recipient_sent_paginated(
            if violating_usc18_1702 {
                props.profile.clone()
            } else {
                auth_user.id.clone()
            },
            props.page,
        )
        .await
    {
        Ok(c) => c,
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
        payload: crate::routing::into_some_serde_value(OutboxTemplate {
            mail,
            page: props.page,
            violating_usc18_1702,
            pid: if violating_usc18_1702 {
                props.profile.clone()
            } else {
                auth_user.id.clone()
            },
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ViewTemplate {
    letter: Mail,
    author: Box<Profile>,
    is_helper: bool,
}

/// GET /inbox/mail/letter/{id}
pub async fn view_request(
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

    let letter = match database.auth.get_mail(id).await {
        Ok(c) => c,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let author = match database.auth.get_profile(letter.author.clone()).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    // check view permission
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

    if !group.permissions.contains(&Permission::Manager) {
        // make sure we're a recipient or the author
        if !letter.recipient.contains(&auth_user.id) && auth_user.id != author.id {
            return Json(AuthError::NotAllowed.to_json());
        }
    }

    // ...
    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ViewTemplate {
            letter,
            author,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PartialMailTemplate {
    profile: Option<Box<Profile>>,
    lang: langbeam::LangFile,
    letter: Mail,
    author: Box<Profile>,
    is_helper: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PartialMailProps {
    pub id: String,
}

/// GET /inbox/mail/_app/components/mail.html
pub async fn partial_mail_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(props): Query<PartialMailProps>,
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

    let letter = match database.auth.get_mail(props.id.clone()).await {
        Ok(r) => r,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    let author = match database.auth.get_profile(letter.author.clone()).await {
        Ok(ua) => ua,
        Err(e) => {
            return Json(DefaultReturn {
                success: false,
                message: e.to_string(),
                payload: None,
            })
        }
    };

    // check view permission
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

    if !group.permissions.contains(&Permission::Manager) {
        // make sure we're a recipient or the author
        if !letter.recipient.contains(&auth_user.id) && auth_user.id != author.id {
            return Json(AuthError::NotAllowed.to_json());
        }
    }

    // ...
    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    };

    // ...
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PartialMailTemplate {
            profile: Some(auth_user),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            letter,
            author,
            is_helper,
        }),
    })
}
