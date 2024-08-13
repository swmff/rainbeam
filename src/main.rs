//! Sparkler server
use axum::routing::{get, get_service};
use axum::Router;

use xsu_authman::{Database as AuthDatabase, api as AuthApi};
use xsu_dataman::config::Config as DataConf;

mod config;
mod database;
mod model;
mod routing;

/// Main server process
#[tokio::main]
pub async fn main() {
    let mut config = config::Config::get_config();

    let home = std::env::var("HOME").expect("failed to read $HOME");
    let static_dir = format!("{home}/.config/xsu-apps/sparkler/static");
    config.static_dir = static_dir.clone();

    // create databases
    let auth_database = AuthDatabase::new(
        DataConf::get_config().connection, // pull connection config from config file
        xsu_authman::ServerOptions::truthy(),
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
        .nest_service("/api/auth", AuthApi::routes(auth_database.clone()))
        .nest("/api/v1", routing::api::routes(database.clone()))
        // pages
        .nest("/", routing::pages::routes(database.clone()).await)
        // ...
        .nest_service(
            "/static",
            get_service(tower_http::services::ServeDir::new(static_dir)),
        )
        .fallback_service(get(routing::pages::not_found).with_state(database.clone()));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", config.port))
        .await
        .unwrap();

    println!("Starting server at http://localhost:{}!", config.port);
    axum::serve(listener, app).await.unwrap();
}
