mod endpoints;
pub mod error;
mod api_models;

use axum::extract::DefaultBodyLimit;
use std::env;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use tokio::{self, net::TcpListener};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use tracing_subscriber::EnvFilter;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
struct AppState {
    conn: Pool<Postgres>,
    storage_dir: std::path::PathBuf,
    thumbnail_dir: std::path::PathBuf,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // unsafe {
    //     env::set_var("RUST_LOG", "debug");
    // }
    // tracing_subscriber::fmt::init();

    tracing_subscriber::fmt()
        // This allows you to use, e.g., `RUST_LOG=info` or `RUST_LOG=debug`
        // when running the app to set log levels.
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("trace"))?,
        )
        .init();

    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{host}:{port}");
    println!("server_url: {}", server_url);
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url.as_str()).await?;
    

    let state = AppState {
        conn: pool,
        storage_dir: env::var("STORAGE")
            .expect("STORAGE is not set in .env file")
            .parse()?,
        thumbnail_dir: env::var("THUMBNAILS")
            .expect("THUMBNAILS is not set in .env file")
            .parse()?,
    };

    let (router, api) = OpenApiRouter::new()
        .routes(routes!(endpoints::media::post_media))
        // .routes(routes!(endpoints::media::update_media_item))
        .routes(routes!(endpoints::media::media_item_patch))
        .routes(routes!(endpoints::media::get_media_item))
        .routes(routes!(endpoints::media::get_media_file))
        .routes(routes!(endpoints::media::get_media_thumbnail))
        .routes(routes!(endpoints::media::get_media_item_by_hash))
        .routes(routes!(endpoints::media::delete_media_item))
        .routes(routes!(endpoints::media::get_media_item_creators))
        .routes(routes!(endpoints::media::get_media_item_collections))
        .routes(routes!(endpoints::media::get_media_item_tags))

        .routes(routes!(endpoints::search::search_query))
        .routes(routes!(endpoints::search::search_query_json))
        .routes(routes!(endpoints::search::hash_search))

        .routes(routes!(endpoints::tags::search_tags))
        .routes(routes!(endpoints::autocomplete::autocomplete))
        .routes(routes!(endpoints::collection::get_collections))
        .routes(routes!(endpoints::collection::get_collection_id))
        .routes(routes!(endpoints::collection::patch_collection_id))
        .routes(routes!(endpoints::collection::collection_id_add))
        .routes(routes!(endpoints::collection::post_collection))
        .routes(routes!(endpoints::collection::get_collection_path))
        .routes(routes!(endpoints::collection::get_collection_id_thumbnail))

        .routes(routes!(endpoints::creators::get_creators))
        .routes(routes!(endpoints::creators::get_creators_id))
        .routes(routes!(endpoints::creators::get_creators_by_alias))
        .routes(routes!(endpoints::creators::patch_creators_id))
        
        .routes(routes!(endpoints::duplicates::get_duplicates))




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
