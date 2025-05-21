mod endpoints;
pub mod error;
mod queries;
mod api_models;

use axum::extract::DefaultBodyLimit;
use sea_orm::{Database, DatabaseConnection};
use std::env;
use tokio::{self, net::TcpListener};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

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

    let (router, api) = OpenApiRouter::new()
        .routes(routes!(endpoints::media::get_media))
        .routes(routes!(endpoints::media::post_media))
        .routes(routes!(endpoints::media::update_media_item))
        .routes(routes!(endpoints::media::media_item_patch))
        .routes(routes!(endpoints::media::get_media_item))
        .routes(routes!(endpoints::media::get_media_file))
        .routes(routes!(endpoints::media::get_media_thumbnail))
        .routes(routes!(endpoints::media::get_media_item_by_hash))
        .routes(routes!(endpoints::search::search_query))
        .routes(routes!(endpoints::tags::search_tags))
        .routes(routes!(endpoints::autocomplete::autocomplete))
        .routes(routes!(endpoints::collection::get_collections))
        .routes(routes!(endpoints::collection::get_collection_id))
        .routes(routes!(endpoints::collection::patch_collection_id))
        .routes(routes!(endpoints::collection::collection_id_add))
        .routes(routes!(endpoints::collection::post_collection))
        .routes(routes!(endpoints::creators::get_creators))
        .routes(routes!(endpoints::creators::get_creators_id))
        .routes(routes!(endpoints::creators::get_creators_collection))
        .routes(routes!(endpoints::creators::get_creators_uncollected))
        .routes(routes!(endpoints::creators::get_creators_media))




        .split_for_parts();

    let app = router
        .merge(SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", api.clone()))
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
