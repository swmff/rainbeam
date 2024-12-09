//! 🌈 Rainbeam blogging service
#![doc = include_str!("../../../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/swmff/rainbeam/issues")]
#![doc(html_favicon_url = "https://rainbeam.net/static/favicon.svg")]
#![doc(html_logo_url = "https://rainbeam.net/static/favicon.svg")]
use askama_axum::Template;
use axum::routing::{get, get_service};
use axum::Router;

use tower_http::trace::{self, TraceLayer};
use tracing::{info, Level};

use authbeam::{api as AuthApi, Database as AuthDatabase};
use databeam::config::Config as DataConf;
use shared::fs;

pub use blogbeam::database;
pub use blogbeam::model;
pub use rb::config;
mod routes;

// mimalloc
#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// ...
/// Trait to convert errors into HTML
pub(crate) trait ToHtml {
    fn to_html(&self, database: database::Database) -> String;
}

impl ToHtml for model::DatabaseError {
    fn to_html(&self, database: database::Database) -> String {
        crate::routes::ErrorTemplate {
            config: database.server_options.clone(),
            lang: database.lang("net.rainbeam.langs:en-US"),
            profile: None,
            message: self.to_string(),
        }
        .render()
        .unwrap()
    }
}

/// Main server process
#[tokio::main]
pub async fn main() {
    let mut config = config::Config::get_config();

    let c = fs::canonicalize(".").unwrap();
    let here = c.to_str().unwrap();

    let static_dir = format!("{here}/.config/static");
    config.static_dir = static_dir.clone();

    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // create databases
    let auth_database = AuthDatabase::new(
        DataConf::get_config().connection, // pull connection config from config file
        authbeam::ServerOptions {
            captcha: config.captcha.clone(),
            registration_enabled: config.registration_enabled,
            real_ip_header: config.real_ip_header.clone(),
            static_dir: config.static_dir.clone(),
            host: config.host.clone(),
            citrus_id: config.citrus_id.clone(),
            blocked_hosts: config.blocked_hosts.clone(),
            secure: config.secure.clone(),
        },
    )
    .await;
    auth_database.init().await;

    let rb_database = rb::database::Database::new(
        DataConf::get_config().connection,
        auth_database.clone(),
        config.clone(),
    )
    .await;
    rb_database.init().await;

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
        .nest("/api/v1", blogbeam::api::routes(database.clone()))
        // pages
        .nest("/", routes::routes(database.clone()).await)
        // auth pages
        .nest("/_rb", routes::rb_external(rb_database.clone()).await)
        // ...
        .nest_service(
            "/static",
            get_service(tower_http::services::ServeDir::new(&static_dir)),
        )
        .fallback_service(get(routes::not_found).with_state(database.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", config.port))
        .await
        .unwrap();

    info!("🌈 Starting server at: http://localhost:{}!", config.port);
    axum::serve(listener, app).await.unwrap();
}
