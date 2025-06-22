use crate::api_models::{
    ApiCollectionResult, ApiMediaReturn, HashQuery, Pagination, QueryType, SearchQuery,
    SearchQueryJson, SearchResult,
};
use crate::error::AppError;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Query;
use chrono::FixedOffset;
use sqlx::types::BitVec;
use std::collections::HashMap;

#[utoipa::path(get, path = "/v1/search", params(SearchQuery, Pagination), responses((status = OK, body = SearchResult)), tags = ["search"]
)]
pub async fn search_query(
    state: State<AppState>,
    query: Query<SearchQuery>,
    pagination: Query<Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    let collections_include: Vec<String> = Vec::new();
    let collections_exclude: Vec<String> = Vec::new();
    let mut creators_include: Vec<String> = Vec::new();
    let creators_exclude: Vec<String> = Vec::new();
    let mut tags_include: Vec<String> = Vec::new();
    let mut tags_exclude: Vec<String> = Vec::new();
    dbg!(&query);
    if !query.tags.is_empty() {
        tags_include.extend(
            query
                .tags
                .clone()
                .into_iter()
                .filter(|x| !x.starts_with('-')),
        );
        tags_exclude.extend(
            query
                .tags
                .clone()
                .into_iter()
                .filter(|x| x.starts_with('-'))
                .map(|t| t.replacen("-", "", 1)),
        );
    }

    creators_include.extend(query.creators.iter().map(|i| i.to_string()));

    let r = sqlx::query_file_scalar!(
        "sql/endpoints/search/search.sqlx",
        &creators_include[..],
        &creators_exclude[..],
        false,
        &collections_include[..],
        &collections_exclude[..],
        false,
        &tags_include[..],
        &tags_exclude[..],
        false,
        pagination.per_page.unwrap_or(20).cast_signed(),
        pagination.last.unwrap_or(0).cast_signed(),
    )
    .fetch_all(&state.conn)
    .await?;

    let perceptual_hash: Option<BitVec> = None;

    Ok((
        StatusCode::OK,
        Json(SearchResult {
            result: sqlx::query_file_as!(
                ApiMediaReturn,
                "sql/media_item_get.sqlx",
                &r[..],
                perceptual_hash,
            )
            .fetch_all(&state.conn)
            .await?,
            ..Default::default()
        }),
    ))
}

async fn query_media(
    tags: Option<Vec<String>>,
    creators: Option<Vec<String>>,
    collections: Option<Vec<String>>,
    pagination: Pagination,
    db: &sqlx::PgPool,
) -> Result<Vec<ApiMediaReturn>, AppError> {
    let mut collections_include: Vec<String> = Vec::new();
    let mut collections_exclude: Vec<String> = Vec::new();
    let mut no_collections: bool = false;
    let mut creators_include: Vec<String> = Vec::new();
    let mut creators_exclude: Vec<String> = Vec::new();
    let mut no_creators: bool = false;
    let mut tags_include: Vec<String> = Vec::new();
    let mut tags_exclude: Vec<String> = Vec::new();
    let mut no_tags: bool = false;

    if let Some(collections) = collections {
        collections_include.extend(
            collections
                .iter()
                .filter(|i| !i.starts_with("-"))
                .map(|i| i.to_string()),
        );
        collections_exclude.extend(
            collections
                .iter()
                .filter(|i| i.starts_with("-"))
                .map(|i| i.to_string()),
        );
    } else {
        no_collections = true;
    }

    if let Some(creators) = creators {
        creators_include.extend(
            creators
                .iter()
                .filter(|i| !i.starts_with("-"))
                .map(|i| i.to_string()),
        );
        creators_exclude.extend(
            creators
                .iter()
                .filter(|i| i.starts_with("-"))
                .map(|i| i.to_string()),
        );
    } else {
        no_creators = true;
    }

    if let Some(tags) = tags {
        tags_include.extend(
            tags.iter()
                .filter(|i| !i.starts_with("-"))
                .map(|i| i.to_string()),
        );
        tags_exclude.extend(
            tags.iter()
                .filter(|i| i.starts_with("-"))
                .map(|i| i.to_string()),
        );
    } else {
        no_tags = true;
    }
    let r = sqlx::query_file_scalar!(
        "sql/endpoints/search/search.sqlx",
        &creators_include[..],
        &creators_exclude[..],
        no_creators,
        &collections_include[..],
        &collections_exclude[..],
        no_collections,
        &tags_include[..],
        &tags_exclude[..],
        no_tags,
        pagination.per_page.unwrap_or(20).cast_signed(),
        pagination.last.unwrap_or(0).cast_signed(),
    )
    .fetch_all(db)
    .await?;

    let perceptual_hash: Option<BitVec> = None;
    Ok(sqlx::query_file_as!(
        ApiMediaReturn,
        "sql/media_item_get.sqlx",
        &r[..],
        perceptual_hash,
    )
    .fetch_all(db)
    .await?)
}

async fn query_collections(
    tags: Option<Vec<String>>,
    creators: Option<Vec<String>>,
    pagination: Pagination,
    db: &sqlx::PgPool,
) -> Result<Vec<ApiCollectionResult>, AppError> {
    let mut creators_include: Vec<String> = Vec::new();
    let mut creators_exclude: Vec<String> = Vec::new();
    let mut tags_include: Vec<String> = Vec::new();
    let mut tags_exclude: Vec<String> = Vec::new();

    if let Some(creators) = creators {
        creators_include.extend(
            creators
                .iter()
                .filter(|i| !i.starts_with("-"))
                .map(|i| i.to_string()),
        );
        creators_exclude.extend(
            creators
                .iter()
                .filter(|i| i.starts_with("-"))
                .map(|i| i.to_string()),
        );
    }

    if let Some(tags) = tags {
        tags_include.extend(
            tags.iter()
                .filter(|i| !i.starts_with("-"))
                .map(|i| i.to_string()),
        );
        tags_exclude.extend(
            tags.iter()
                .filter(|i| i.starts_with("-"))
                .map(|i| i.to_string()),
        );
    }

    Ok(sqlx::query_file_as!(
        ApiCollectionResult,
        "sql/endpoints/search/collection.sqlx",
        &creators_include[..],
        &creators_exclude[..],
        &tags_include[..],
        &tags_exclude[..],
        pagination.per_page.unwrap_or(20).cast_signed(),
        pagination.last.unwrap_or(0).cast_signed()
    )
    .fetch_all(db)
    .await?)
}

#[utoipa::path(post, path = "/v1/search", params(Pagination), request_body = SearchQueryJson, responses((status = OK, body = SearchResult)), tags = ["search"]
)]
pub async fn search_query_json(
    state: State<AppState>,
    pagination: Query<Pagination>,
    query: Json<SearchQueryJson>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    let mut media: Vec<ApiMediaReturn> = vec![];
    let mut collections: Option<Vec<ApiCollectionResult>> = None;
    dbg!(&query);
    match query.query_type {
        QueryType::All => {
            media = query_media(
                query.tags.clone(),
                query.creators.clone(),
                query.collections.clone(),
                pagination.0.clone(),
                &state.conn,
            )
            .await?;
            collections = Some(
                query_collections(
                    query.tags.clone(),
                    query.creators.clone(),
                    pagination.0.clone(),
                    &state.conn,
                )
                .await?,
            )
        }
        QueryType::Media => {
            media = query_media(
                query.tags.clone(),
                query.creators.clone(),
                query.collections.clone(),
                pagination.0,
                &state.conn,
            )
            .await?;
        }
        QueryType::Collection => {
            collections = Some(
                query_collections(
                    query.tags.clone(),
                    query.creators.clone(),
                    pagination.0.clone(),
                    &state.conn,
                )
                .await?,
            )
        }
    }
    //
    Ok((
        StatusCode::OK,
        Json(SearchResult {
            result: media,
            collections,
            ..Default::default()
        }),
    ))
}

#[utoipa::path(get, path = "/v1/search/hash", params(HashQuery, Pagination), responses((status = OK, body = SearchResult)), tags = ["search"]
)]
pub async fn hash_search(
    state: State<AppState>,
    query: Query<HashQuery>,
    pagination: Query<Pagination>,
) -> Result<Json<SearchResult>, AppError> {
    let r = sqlx::query_file_scalar!(
        "sql/endpoints/search/hash_search.sqlx",
        BitVec::from_bytes(&query.hash.to_be_bytes()),
        query.max_distance.unwrap_or(3) as f64,
        pagination.per_page.unwrap_or(20).cast_signed(),
        pagination.last.unwrap_or(0).cast_signed()
    )
    .fetch_all(&state.conn)
    .await?;

    let found_media = sqlx::query_file_as!(
        ApiMediaReturn,
        "sql/media_item_get.sqlx",
        &r[..],
        BitVec::from_bytes(&query.hash.to_be_bytes()),
    )
    .fetch_all(&state.conn)
    .await?;

    Ok(Json(SearchResult {
        result: found_media,
        ..Default::default()
    }))
}
