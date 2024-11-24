use askama_axum::Template;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_extra::extract::CookieJar;

use authbeam::model::{Mail, Profile, Permission};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::database::Database;
use crate::model::DatabaseError;
use super::NotificationsQuery;
use crate::ToHtml;

#[derive(Template)]
#[template(path = "mail/inbox.html")]
struct InboxTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    mail: Vec<(Mail, Profile)>,
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

    let violating_usc18_1702 =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if violating_usc18_1702 && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
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
        Err(e) => return Html(e.to_string()),
    };

    Html(
        InboxTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            mail,
            page: props.page,
            violating_usc18_1702,
            pid: if violating_usc18_1702 {
                props.profile.clone()
            } else {
                auth_user.id.clone()
            },
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "mail/outbox.html")]
struct OutboxTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    mail: Vec<(Mail, Profile)>,
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

    let violating_usc18_1702 =
        (props.profile.is_empty() == false) && (props.profile != auth_user.id);

    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.contains(&Permission::Helper)
    };

    if violating_usc18_1702 && !is_helper {
        // we cannot view the mail of other users if we are not a helper
        return Html(DatabaseError::NotAllowed.to_html(database));
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
        Err(e) => return Html(e.to_string()),
    };

    Html(
        OutboxTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            mail,
            page: props.page,
            violating_usc18_1702,
            pid: if violating_usc18_1702 {
                props.profile.clone()
            } else {
                auth_user.id.clone()
            },
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "mail/compose.html")]
struct ComposeTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
}

/// GET /inbox/mail/compose
pub async fn compose_request(
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
        ComposeTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "mail/view.html")]
struct ViewTemplate {
    config: Config,
    profile: Option<Profile>,
    unread: usize,
    notifs: usize,
    letter: Mail,
    author: Profile,
}

/// GET /inbox/mail/letter/:id
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

    let letter = match database.auth.get_mail(id).await {
        Ok(c) => c,
        Err(e) => return Html(e.to_string()),
    };

    let author = match database.auth.get_profile(letter.author.clone()).await {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_string()),
    };

    Html(
        ViewTemplate {
            config: database.server_options.clone(),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            letter,
            author,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "mail/components/mail.html")]
struct PartialMailTemplate {
    profile: Option<Profile>,
    letter: Mail,
    author: Profile,
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
            Ok(ua) => Some(ua),
            Err(_) => None,
        },
        None => None,
    };

    let letter = match database.auth.get_mail(props.id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_string()),
    };

    let author = match database.auth.get_profile(letter.author.clone()).await {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_string()),
    };

    // ...
    Html(
        PartialMailTemplate {
            profile: auth_user,
            letter,
            author,
        }
        .render()
        .unwrap(),
    )
}
