mod endpoints;
pub mod error;
mod queries;

use axum::Router;
use axum::routing::get;
use sea_orm::{Database, DatabaseConnection};
use std::env;
use axum::extract::DefaultBodyLimit;
use tokio::{self, net::TcpListener};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

#[derive(Clone)]
struct AppState {
    conn: DatabaseConnection,
    storage_dir: std::path::PathBuf,
    thumbnail_dir: std::path::PathBuf,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    unsafe {
        env::set_var("RUST_LOG", "debug");
    }
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{host}:{port}");
    println!("server_url: {}", server_url);

    let conn = Database::connect(db_url)
        .await
        .expect("Database connection failed");

    let state = AppState {
        conn,
        storage_dir: env::var("STORAGE")
            .expect("STORAGE is not set in .env file")
            .parse()?,
        thumbnail_dir: env::var("THUMBNAILS")
            .expect("THUMBNAILS is not set in .env file")
            .parse()?,
    };

    let app = Router::new()
        .route(
            "/v1/media",
            get(endpoints::media::get_media).post(endpoints::media::post_media),
        )
        .route(
            "/v1/media/{id}",
            get(endpoints::media::get_media_item).put(endpoints::media::update_media_item),
        )
        .route("/v1/media/{id}/file", get(endpoints::media::get_media_file))
        .route("/v1/media/{id}/thumbnail", get(endpoints::media::get_media_thumbnail))
        .route(
            "/v1/media/{id}/tags",
            get(endpoints::media::media_get_tags)
                .put(endpoints::media::media_add_tag)
                .delete(endpoints::media::media_delete_tag),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(DefaultBodyLimit::max(1024*1024*1024*10))
        .with_state(state);

    let listener = TcpListener::bind(&server_url).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
