use authbeam::api::profile::read_image;
use crate::database::Database;
use askama_axum::IntoResponse;
use axum::{
    body::Body,
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn routes(database: Database) -> Router {
    Router::new()
        .route("/lang", get(langfile_request))
        .route("/lang/set", post(set_langfile_request))
        .route("/ext/image", get(external_image_request))
        // ...
        .with_state(database.clone())
}

#[derive(Serialize, Deserialize)]
pub struct ExternalImageQuery {
    pub img: String,
}

/// Proxy an external image
pub async fn external_image_request(
    Query(props): Query<ExternalImageQuery>,
    State(database): State<Database>,
) -> impl IntoResponse {
    let image_url = &props.img;

    if image_url.starts_with(&database.config.host) {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.config.static_dir,
                "default-banner.svg".to_string(),
            )),
        );
    }

    for host in database.config.blocked_hosts {
        if image_url.starts_with(&host) {
            return (
                [("Content-Type", "image/svg+xml")],
                Body::from(read_image(
                    database.config.static_dir,
                    "default-banner.svg".to_string(),
                )),
            );
        }
    }

    // get profile image
    if image_url.is_empty() {
        return (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.config.static_dir,
                "default-banner.svg".to_string(),
            )),
        );
    }

    let guessed_mime = mime_guess::from_path(image_url)
        .first_raw()
        .unwrap_or("application/octet-stream");

    match database.auth.http.get(image_url).send().await {
        Ok(stream) => {
            if let Some(ct) = stream.headers().get("Content-Type") {
                let bad_ct = vec!["text/html", "text/plain"];
                if bad_ct.contains(&ct.to_str().unwrap()) {
                    // if we got html, return default banner (likely an error page)
                    return (
                        [("Content-Type", "image/svg+xml")],
                        Body::from(read_image(
                            database.config.static_dir,
                            "default-banner.svg".to_string(),
                        )),
                    );
                }
            }

            (
                [(
                    "Content-Type",
                    if guessed_mime == "text/html" {
                        "text/plain"
                    } else {
                        guessed_mime
                    },
                )],
                Body::from_stream(stream.bytes_stream()),
            )
        }
        Err(_) => (
            [("Content-Type", "image/svg+xml")],
            Body::from(read_image(
                database.config.static_dir,
                "default-banner.svg".to_string(),
            )),
        ),
    }
}

#[derive(Serialize, Deserialize)]
pub struct LangFileQuery {
    #[serde(default)]
    pub id: String,
}

/// Get a langfile
pub async fn langfile_request(
    Query(props): Query<LangFileQuery>,
    State(database): State<Database>,
) -> impl IntoResponse {
    Json(database.lang(&props.id))
}

/// Set a langfile
pub async fn set_langfile_request(Query(props): Query<LangFileQuery>) -> impl IntoResponse {
    (
        {
            let mut headers = HeaderMap::new();

            headers.insert(
                "Set-Cookie",
                format!(
                    "net.rainbeam.langs.choice={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}",
                    props.id,
                    60* 60 * 24 * 365
                )
                .parse()
                .unwrap(),
            );

            headers
        },
        "Language changed",
    )
}
