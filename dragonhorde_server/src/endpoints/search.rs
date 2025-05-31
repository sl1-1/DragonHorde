use crate::api_models::{ApiMedia, HashQuery, Pagination, SearchQuery, SearchResult};
use crate::error::AppError;
use crate::queries::{base_media, distance, media_from_search, search_creator, search_hash};
use crate::{queries, AppState};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Query;
use sea_orm::{ConnectionTrait, FromQueryResult};

#[utoipa::path(get, path = "/v1/search", params(SearchQuery, Pagination), responses((status = OK, body = SearchResult)), tags = ["search"]
)]
pub async fn search_query(
    state: State<AppState>,
    query: Query<SearchQuery>,
    pagination: Query<Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    dbg!(&query);
    let mut q = queries::base_search_query();
    if !query.tags.is_empty() {
        let tags: Vec<String> = query
            .tags
            .clone()
            .into_iter()
            .filter(|x| !x.starts_with('-'))
            .collect();
        let blocked: Vec<String> = query
            .tags
            .clone()
            .into_iter()
            .filter(|x| x.starts_with('-'))
            .map(|t| t.replacen("-", "", 1))
            .collect();
        if !tags.is_empty() {
            q = queries::search_has_tags(q, tags);
        }
        if !blocked.is_empty() {
            q = queries::search_not_tags(q, blocked);
        }
    }
    if !query.creators.is_empty() {
        q = search_creator(q, query.creators.clone());
    }
    q = queries::pagination(q, pagination.0);
    let mut media_q = base_media();
    media_q = media_from_search(media_q, q);
    
    let statement = state.conn.get_database_backend().build(&media_q);
    let found_media = ApiMedia::find_by_statement(statement)
        .all(&state.conn)
        .await?;
    Ok((
        StatusCode::OK,
        Json(SearchResult {
            result: found_media,
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
    dbg!(&query);
    let mut q = queries::base_search_query();
    q = search_hash(q, query.hash, query.max_distance);
    q = queries::pagination(q, pagination.0);
    let mut media_q = base_media();
    media_q = media_from_search(media_q, q);
    media_q = distance(media_q, query.hash);
    let statement = state.conn.get_database_backend().build(&media_q);
    let found_media = ApiMedia::find_by_statement(statement)
        .all(&state.conn)
        .await?;
    Ok(
        Json(SearchResult {
            result: found_media,
        }),
    )
}