use crate::endpoints::media::{ApiMedia, SearchResult};
use crate::error::AppError;
use crate::{AppState, queries};
use axum::Json;
use axum::extract::{State};
use axum_extra::extract::Query;
use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, FromQueryResult};
use serde::Deserialize;
use crate::queries::search_creator;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    has_tags: Vec<String>,
    #[serde(default)]
    not_tags: Vec<String>,
    #[serde(default)]
    creators: Vec<String>,
}

pub async fn search_query(
    state: State<AppState>,
    query: Query<SearchQuery>,
    pagination: Query<crate::endpoints::media::Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    dbg!(&query);
    let mut q = queries::base_media();
    if !query.has_tags.is_empty() {
       q =  queries::search_has_tags(q, query.has_tags.clone());
    }
    if !query.not_tags.is_empty() {
        q = queries::search_not_tags(q, query.not_tags.clone());
    }
    if !query.creators.is_empty() {
        q = search_creator(q, query.creators.clone());
    }
    q = queries::pagination(q, pagination.0);
    let statement = state
        .conn
        .get_database_backend()
        .build(&q);
    let found_media = ApiMedia::find_by_statement(statement).all(&state.conn).await?;
    Ok((
        StatusCode::OK,
        Json(SearchResult {
            result: found_media,
        }),
    ))
}
