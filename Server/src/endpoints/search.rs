use crate::endpoints::media::{ApiMedia, SearchResult};
use crate::error::AppError;
use crate::{AppState, queries};
use axum::Json;
use axum::extract::{State};
use axum_extra::extract::Query;
use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, FromQueryResult};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    has_tags: Vec<String>,
    not_tags: Vec<String>,
}

pub async fn search_query(
    state: State<AppState>,
    query: Query<SearchQuery>,
    pagination: Query<crate::endpoints::media::Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    dbg!(&query);
    let q = state
        .conn
        .get_database_backend()
        .build(&queries::search_query(
            if !query.has_tags.is_empty() {Some(query.has_tags.clone())} else {None},
            if !query.not_tags.is_empty() {Some(query.not_tags.clone())} else {None},
            Some(pagination.0),
        ));
    let found_media = ApiMedia::find_by_statement(q).all(&state.conn).await?;
    Ok((
        StatusCode::OK,
        Json(SearchResult {
            result: found_media,
        }),
    ))
}
