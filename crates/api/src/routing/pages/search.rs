use std::collections::HashMap;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::extract::State;
use axum::Json;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use authbeam::model::{Permission, Profile, RelationshipStatus};

use databeam::DefaultReturn;
use super::{SearchHomeQuery, SearchQuery};
use crate::database::Database;
use crate::model::{DatabaseError, FullResponse, Question};

#[derive(Serialize, Deserialize)]
struct HomepageTemplate {
    query: String,
    driver: i8,
}

/// GET /search
pub async fn search_homepage_request(Query(query): Query<SearchHomeQuery>) -> impl IntoResponse {
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(HomepageTemplate {
            query: String::new(),
            driver: query.driver,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct ResponsesTemplate {
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
}

/// GET /search/responses
pub async fn search_responses_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
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

    // search results
    let results = if query.tag.is_empty() {
        match database
            .get_responses_searched_paginated(query.page, query.q.clone())
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Json(e.to_json()),
        }
    } else {
        match database
            .get_responses_tagged_paginated(query.tag.clone(), query.page)
            .await
        {
            Ok(responses) => responses,
            Err(e) => return Json(e.to_json()),
        }
    };

    // permissions
    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    if let Some(ref ua) = auth_user {
        for response in &results {
            if relationships.contains_key(&response.1.author.id) {
                continue;
            }

            if response.1.author.id == ua.id {
                // make sure we can view our own responses
                relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
                continue;
            };

            relationships.insert(
                response.1.author.id.clone(),
                database
                    .auth
                    .get_user_relationship(response.1.author.id.clone(), ua.id.clone())
                    .await
                    .0,
            );
        }
    } else {
        for response in &results {
            // no user, no relationships
            if relationships.contains_key(&response.1.author.id) {
                continue;
            }

            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Unknown);
        }
    }

    // render
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(ResponsesTemplate {
            query: query.q,
            page: query.page,
            driver: if query.tag.is_empty() { 0 } else { 4 },
            // search-specific
            results,
            relationships,
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct PostsTemplate {
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<FullResponse>,
    relationships: HashMap<String, RelationshipStatus>,
    is_powerful: bool, // at least "manager"
    is_helper: bool,   // at least "helper"
}

/// GET /search/posts
pub async fn search_posts_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
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

    // search results
    let results = match database
        .get_posts_searched_paginated(query.page, query.q.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    // permissions
    let mut is_helper: bool = false;
    let is_powerful = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        is_helper = group.permissions.contains(&Permission::Helper);
        group.permissions.contains(&Permission::Manager)
    } else {
        false
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    if let Some(ref ua) = auth_user {
        for response in &results {
            if relationships.contains_key(&response.1.author.id) {
                continue;
            }

            if response.1.author.id == ua.id {
                // make sure we can view our own responses
                relationships.insert(response.1.author.id.clone(), RelationshipStatus::Friends);
                continue;
            };

            relationships.insert(
                response.1.author.id.clone(),
                database
                    .auth
                    .get_user_relationship(response.1.author.id.clone(), ua.id.clone())
                    .await
                    .0,
            );
        }
    } else {
        for response in &results {
            // no user, no relationships
            if relationships.contains_key(&response.1.author.id) {
                continue;
            }

            relationships.insert(response.1.author.id.clone(), RelationshipStatus::Unknown);
        }
    }

    // render
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(PostsTemplate {
            query: query.q,
            page: query.page,
            driver: 2,
            // search-specific
            results,
            relationships,
            is_powerful,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct QuestionsTemplate {
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<(Question, usize, usize)>,
    relationships: HashMap<String, RelationshipStatus>,
    is_helper: bool, // at least "helper"
}

/// GET /search/questions
pub async fn search_questions_request(
    jar: CookieJar,
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
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

    // search results
    let results = match database
        .get_global_questions_searched_paginated(query.page, query.q.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    // build relationships list
    let mut relationships: HashMap<String, RelationshipStatus> = HashMap::new();

    if let Some(ref ua) = auth_user {
        for question in &results {
            if relationships.contains_key(&question.0.author.id) {
                continue;
            }

            if question.0.author.id == ua.id {
                // make sure we can view our own questions
                relationships.insert(question.0.author.id.clone(), RelationshipStatus::Friends);
                continue;
            };

            relationships.insert(
                question.0.author.id.clone(),
                database
                    .auth
                    .get_user_relationship(question.0.author.id.clone(), ua.id.clone())
                    .await
                    .0,
            );
        }
    } else {
        for question in &results {
            // no user, no relationships
            if relationships.contains_key(&question.0.author.id) {
                continue;
            }

            relationships.insert(question.0.author.id.clone(), RelationshipStatus::Unknown);
        }
    }

    // permissions
    let is_helper = if let Some(ref ua) = auth_user {
        let group = match database.auth.get_group_by_id(ua.group).await {
            Ok(g) => g,
            Err(_) => return Json(DatabaseError::Other.to_json()),
        };

        group.permissions.contains(&Permission::Helper)
    } else {
        false
    };

    // render
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(QuestionsTemplate {
            query: query.q,
            page: query.page,
            driver: 1,
            // search-specific
            results,
            relationships,
            is_helper,
        }),
    })
}

#[derive(Serialize, Deserialize)]
struct UsersTemplate {
    query: String,
    page: i32,
    driver: i8,
    // search-specific
    results: Vec<Box<Profile>>,
}

/// GET /search/users
pub async fn search_users_request(
    State(database): State<Database>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    // search results
    let results = match database
        .get_profiles_searched_paginated(query.page, query.q.clone())
        .await
    {
        Ok(responses) => responses,
        Err(e) => return Json(e.to_json()),
    };

    // render
    Json(DefaultReturn {
        success: true,
        message: String::new(),
        payload: crate::routing::into_some_serde_value(UsersTemplate {
            query: query.q,
            page: query.page,
            driver: 3,
            // search-specific
            results,
        }),
    })
}
