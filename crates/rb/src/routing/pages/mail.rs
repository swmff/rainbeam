use reva_axum::Template;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_extra::extract::CookieJar;

use authbeam::model::{DatabaseError as AuthError, FinePermission, Mail, Profile};
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
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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

        group.permissions.check_helper()
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
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
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
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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

        group.permissions.check_helper()
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
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
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
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
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
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
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
    lang: langbeam::LangFile,
    profile: Option<Box<Profile>>,
    unread: usize,
    notifs: usize,
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

    // check view permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => return Html(e.to_string()),
    };

    if !group.permissions.check(FinePermission::MANAGE_MAILS) {
        // make sure we're a recipient or the author
        if !letter.recipient.contains(&auth_user.id) && auth_user.id != author.id {
            return Html(AuthError::NotAllowed.to_string());
        }
    }

    // ...
    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    // ...
    Html(
        ViewTemplate {
            config: database.config.clone(),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            profile: Some(auth_user.clone()),
            unread,
            notifs,
            letter,
            author,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}

#[derive(Template)]
#[template(path = "mail/components/mail.html")]
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
            Err(_) => return Html(DatabaseError::NotAllowed.to_html(database)),
        },
        None => return Html(DatabaseError::NotAllowed.to_html(database)),
    };

    let letter = match database.auth.get_mail(props.id.clone()).await {
        Ok(r) => r,
        Err(e) => return Html(e.to_string()),
    };

    let author = match database.auth.get_profile(letter.author.clone()).await {
        Ok(ua) => ua,
        Err(e) => return Html(e.to_string()),
    };

    // check view permission
    let group = match database.auth.get_group_by_id(auth_user.group).await {
        Ok(g) => g,
        Err(e) => return Html(e.to_string()),
    };

    if !group.permissions.check(FinePermission::MANAGE_MAILS) {
        // make sure we're a recipient or the author
        if !letter.recipient.contains(&auth_user.id) && auth_user.id != author.id {
            return Html(AuthError::NotAllowed.to_string());
        }
    }

    // ...
    let is_helper = {
        let group = match database.auth.get_group_by_id(auth_user.group).await {
            Ok(g) => g,
            Err(_) => return Html(DatabaseError::Other.to_html(database)),
        };

        group.permissions.check_helper()
    };

    // ...
    Html(
        PartialMailTemplate {
            profile: Some(auth_user),
            lang: database.lang(if let Some(c) = jar.get("net.rainbeam.langs.choice") {
                c.value_trimmed()
            } else {
                ""
            }),
            letter,
            author,
            is_helper,
        }
        .render()
        .unwrap(),
    )
}
