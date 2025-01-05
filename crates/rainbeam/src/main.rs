//! 🌈 Rainbeam!
#![doc = include_str!("../../../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/swmff/rainbeam/issues")]
#![doc(html_favicon_url = "https://rainbeam.net/static/favicon.svg")]
#![doc(html_logo_url = "https://rainbeam.net/static/favicon.svg")]
use axum::routing::{get, get_service};
use axum::Router;

use tower_http::trace::{self, TraceLayer};
use tracing::{info, Level};

use authbeam::{api as AuthApi, Database as AuthDatabase};
use databeam::config::Config as DataConf;
use rainbeam_shared::fs;
use pathbufd::{PathBufD, pathd};

pub use rb::database;
pub use rb::config;
pub use rb::model;
pub use rb::routing;

// mimalloc
#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Main server process
#[tokio::main]
pub async fn main() {
    let mut config = config::Config::get_config();

    let here = PathBufD::current();
    let static_dir = here.join(".config").join("static");
    let well_known_dir = here.join(".config").join(".well-known");
    config.static_dir = static_dir.clone();

    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // make sure media dir is created
    // TODO: implement `.is_empty()` on `PathBufD`
    if !config.media_dir.to_string().is_empty() {
        fs::mkdir(&config.media_dir).expect("failed to create media dir");
        fs::mkdir(pathd!("{}/avatars", config.media_dir)).expect("failed to create avatars dir");
        fs::mkdir(pathd!("{}/banners", config.media_dir)).expect("failed to create banners dir");
    }

    // create databases
    let auth_database = AuthDatabase::new(
        DataConf::get_config().connection, // pull connection config from config file
        authbeam::ServerOptions {
            captcha: config.captcha.clone(),
            registration_enabled: config.registration_enabled,
            real_ip_header: config.real_ip_header.clone(),
            static_dir: config.static_dir.clone(),
            media_dir: config.media_dir.clone(),
            host: config.host.clone(),
            snowflake_server_id: config.snowflake_server_id.clone(),
            blocked_hosts: config.blocked_hosts.clone(),
        },
    )
    .await;
    auth_database.init().await;

    let database = database::Database::new(
        DataConf::get_config().connection,
        auth_database.clone(),
        config.clone(),
    )
    .await;
    database.init().await;

    // create app
    let app = Router::new()
        // api
        .nest_service("/api/v0/auth", AuthApi::routes(auth_database.clone()))
        .nest("/api/v0/util", routing::api::util::routes(database.clone()))
        .nest("/api/v1", routing::api::routes(database.clone()))
        // pages
        .merge(routing::pages::routes(database.clone()).await)
        // ...
        .nest_service(
            "/.well-known",
            get_service(tower_http::services::ServeDir::new(&well_known_dir)),
        )
        .nest_service(
            "/static",
            get_service(tower_http::services::ServeDir::new(&static_dir)),
        )
        .nest_service(
            "/manifest.json",
            get_service(tower_http::services::ServeFile::new(format!(
                "{static_dir}/manifest.json"
            ))),
        )
        .fallback_service(get(routing::pages::not_found).with_state(database.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();

    info!("🌈 Starting server at: http://localhost:{}!", config.port);
    axum::serve(listener, app).await.unwrap();
}
