mod config;
mod db;
mod error;
mod handlers;
mod models;
mod reference;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, put};
use axum::Router;
use tera::Tera;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env();

    std::fs::create_dir_all(&config.image_dir).expect("failed to create image dir");

    let pool = db::init_pool(&config.database_url).await;

    let tera = Tera::new("templates/**/*.html").expect("failed to load templates");

    let state = AppState {
        pool,
        tera: Arc::new(RwLock::new(tera)),
        image_dir: PathBuf::from(&config.image_dir),
    };

    let app = Router::new()
        .route("/", get(handlers::index::index))
        .route(
            "/spools",
            get(handlers::spools::list).post(handlers::spools::create),
        )
        .route("/spools/new", get(handlers::spools::new_form))
        .layer(DefaultBodyLimit::max(25 * 1024 * 1024))
        .route("/spools/:id", get(handlers::spools::detail))
        .route("/spools/:id/weigh", axum::routing::post(handlers::spools::weigh))
        .route(
            "/brands",
            get(handlers::brands::page).post(handlers::brands::create),
        )
        .route(
            "/brands/:id",
            put(handlers::brands::update).delete(handlers::brands::delete),
        )
        .route("/brands/:id/edit", get(handlers::brands::edit))
        .route("/brands/:id/view", get(handlers::brands::view))
        .route(
            "/materials",
            get(handlers::materials::page).post(handlers::materials::create),
        )
        .route(
            "/materials/:id",
            put(handlers::materials::update).delete(handlers::materials::delete),
        )
        .route("/materials/:id/edit", get(handlers::materials::edit))
        .route("/materials/:id/view", get(handlers::materials::view))
        .route(
            "/colours",
            get(handlers::colours::page).post(handlers::colours::create),
        )
        .route(
            "/colours/:id",
            put(handlers::colours::update).delete(handlers::colours::delete),
        )
        .route("/colours/:id/edit", get(handlers::colours::edit))
        .route("/colours/:id/view", get(handlers::colours::view))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/images", ServeDir::new(&config.image_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
