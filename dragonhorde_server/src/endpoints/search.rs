use crate::api_models::{
    ApiCollection, ApiMedia, HashQuery, Pagination, QueryType, SearchQuery,
    SearchQueryJson, SearchResult,
};
use crate::error::AppError;
use crate::queries::{
    base_media, distance, media_from_search, pagination, search_collection_creator,
    search_collection_no_creator, search_collections, search_creator, search_hash,
    search_no_collections, search_no_creator,
};
use crate::{AppState, queries};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum_extra::extract::Query;
use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult};

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
            ..Default::default()
        }),
    ))
}

async fn query_media(
    tags: Option<Vec<String>>,
    creators: Option<Vec<String>>,
    collections: Option<Vec<String>>,
    pagination: Pagination,
    db: &DatabaseConnection,
) -> Result<Vec<ApiMedia>, AppError> {
    let mut q = queries::base_search_query();
    if let Some(tags) = tags {
        if tags.is_empty() {
            let white_list: Vec<String> = tags
                .clone()
                .into_iter()
                .filter(|x| !x.starts_with('-'))
                .collect();
            let black_list: Vec<String> = tags
                .clone()
                .into_iter()
                .filter(|x| x.starts_with('-'))
                .map(|t| t.replacen("-", "", 1))
                .collect();
            if !white_list.is_empty() {
                q = queries::search_has_tags(q, white_list);
            }
            if !black_list.is_empty() {
                q = queries::search_not_tags(q, black_list);
            }
        }
    }
    if let Some(creators) = creators {
        if !creators.is_empty() {
            q = search_creator(q, creators.clone());
        }
    } else {
        q = search_no_creator(q);
    }
    //Todo: Make this either support path style names for collections, or only take IDs?
    if let Some(collections) = collections {
        if !collections.is_empty() {
            q = search_collections(q, collections.clone());
        }
    } else {
        q = search_no_collections(q);
    }

    q = queries::pagination(q, pagination);
    let mut media_q = base_media();
    media_q = media_from_search(media_q, q);

    let statement = db.get_database_backend().build(&media_q);
    Ok(ApiMedia::find_by_statement(statement).all(db).await?)
}

async fn query_collections(
    tags: Option<Vec<String>>,
    creators: Option<Vec<String>>,
    pagination: Pagination,
    db: &DatabaseConnection,
) -> Result<Vec<ApiCollection>, AppError> {
    let mut q = queries::base_collection();
    // if let Some(tags) = tags {
    //     if tags.is_empty() {
    //         let white_list: Vec<String> = tags
    //             .clone()
    //             .into_iter()
    //             .filter(|x| !x.starts_with('-'))
    //             .collect();
    //         let black_list: Vec<String> = tags
    //             .clone()
    //             .into_iter()
    //             .filter(|x| x.starts_with('-'))
    //             .map(|t| t.replacen("-", "", 1))
    //             .collect();
    //         if !white_list.is_empty() {
    //             q = queries::search_has_tags(q, white_list);
    //         }
    //         if !black_list.is_empty() {
    //             q = queries::search_not_tags(q, black_list);
    //         }
    //     }
    // }
    if let Some(creators) = creators {
        if !creators.is_empty() {
            q = search_collection_creator(q, creators.clone());
        }
    } else {
        q = search_collection_no_creator(q);
    }
    // //Todo: Make this either support path style names for collections, or only take IDs?
    // if let Some(collections) = collections {
    //     if !collections.is_empty() {
    //         q = search_collections(q, collections.clone());
    //     }
    // } else {
    //     q = search_no_collections(q);
    // }

    q = queries::pagination(q, pagination);
    // let mut media_q = base_media();
    // media_q = media_from_search(media_q, q);

    let statement = db.get_database_backend().build(&q);
    Ok(ApiCollection::find_by_statement(statement).all(db).await?)
}

#[utoipa::path(post, path = "/v1/search", params(Pagination), request_body = SearchQueryJson, responses((status = OK, body = SearchResult)), tags = ["search"]
)]
pub async fn search_query_json(
    state: State<AppState>,
    pagination: Query<Pagination>,
    query: Json<SearchQueryJson>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    let mut media: Vec<ApiMedia> = vec![];
    let mut collections: Option<Vec<ApiCollection>> = None;
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
    Ok(Json(SearchResult {
        result: found_media,
        ..Default::default()
    }))
}
