use sea_orm::QueryFilter;
use std::collections::HashSet;
pub(crate) use crate::api_models::api_creator::{ApiCreator, CreatorsResults};
use crate::api_models::api_media::ApiMedia;
use crate::api_models::{ApiCollection, CollectionResult, Pagination, SearchResult};
use crate::error::AppError;
use crate::queries::{base_media, collections_by_creator, media_by_creator, media_from_search, media_uncollected};
use crate::{queries, AppState};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use entity::{creators, creators::Entity as Creators};
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, FromQueryResult, Set, TransactionTrait};
use crate::error::AppError::NotFound;
use entity::{creator_alias, creator_alias::Entity as CreatorAlias};


async fn check_creator(id: i64, db: &DatabaseConnection) -> Result<ApiCreator, AppError> {
    let mut q = queries::base_creator();
    q = queries::creator_by_id(q, id);
    let statement = db.get_database_backend().build(&q);
    match ApiCreator::find_by_statement(statement).one(db).await?
    {
        None => Err(NotFound(format!("Creator {} not found", id))),
        Some(c) => Ok(c)
    }
}

#[utoipa::path(get, path = "/v1/creators", responses((status = OK, body = CreatorsResults)), tags = ["creators"])]
pub async fn get_creators(
    state: State<AppState>,
) -> Result<Json<CreatorsResults>, AppError> {
    let q = queries::base_creator();
    let statement = state.conn.get_database_backend().build(&q);
    let creators = ApiCreator::find_by_statement(statement).all(&state.conn).await?;
    Ok(Json(CreatorsResults { result: creators }))
}

#[utoipa::path(get, path = "/v1/creators/{id}", responses((status = OK, body = ApiCreator)), tags = ["creators"])]
pub async fn get_creators_id(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiCreator>, AppError> {
    Ok(Json(check_creator(id, &state.conn).await?))
}

#[utoipa::path(get, path = "/v1/creators/by_alias/{alias}", responses((status = OK, body = ApiCreator)), tags = ["creators"])]
pub async fn get_creators_by_alias(
    state: State<AppState>,
    Path(alias): Path<String>,
) -> Result<Json<ApiCreator>, AppError> {
    let mut q = queries::base_creator();
    q = queries::creator_by_alias(q, &alias);
    let statement = state.conn.get_database_backend().build(&q);
    match ApiCreator::find_by_statement(statement).one(&state.conn).await? {
        None => Err(NotFound(format!("Creator {} not found", alias))),
        Some(c) => Ok(Json(c))
    }
}

#[utoipa::path(patch, path = "/v1/creators/{id}", request_body = ApiCreator, responses((status = OK, body = ApiCreator)), tags = ["creators"])]
pub async fn patch_creators_id(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiCreator>,
) -> Result<Json<ApiCreator>, AppError> {
    let creator = check_creator(id, &state.conn).await?;

    let txn = state.conn.begin().await?;

    if let Some(aliases) = payload.aliases {
        let current_aliases: HashSet<String> = HashSet::from_iter(creator.aliases.unwrap().0);
        let new_aliases: HashSet<String> = HashSet::from_iter(aliases.0);
        let to_delete: Vec<&String> = current_aliases.difference(&new_aliases).collect();
        let to_add: Vec<&String> = new_aliases.difference(&current_aliases).collect();
        if !to_delete.is_empty() {
            CreatorAlias::delete_many()
                .filter(creator_alias::Column::Creator.eq(id))
                .filter(creator_alias::Column::Alias.is_in(to_delete))
                .exec(&txn)
                .await?;
        }
        if !to_add.is_empty() {
            CreatorAlias::insert_many(to_add.into_iter().map(|a| creator_alias::ActiveModel{
                id: Default::default(),
                creator: Set(id),
                alias: Set(a.clone()),
            })).exec(&txn).await?;
        }
    }


    Creators::update( creators::ActiveModel {
        id: Set(id),
        name: Set(payload.name.unwrap_or(creator.name.unwrap())),
        sites: Default::default(),
        created: Default::default(),
    }).exec(&state.conn).await?;

    txn.commit().await?;

    Ok(Json(check_creator(id, &state.conn).await?))
}

#[utoipa::path(get, path = "/v1/creators/{id}/collection", responses((status = OK, body = CollectionResult)), tags = ["creators"])]
pub async fn get_creators_collection(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<CollectionResult>), AppError> {
    let mut q = queries::base_collection();
    q = collections_by_creator(q, id);

    let statement = state.conn.get_database_backend().build(&q);
    let found_collections = ApiCollection::find_by_statement(statement)
        .all(&state.conn)
        .await?;
    Ok((
        StatusCode::OK,
        Json(CollectionResult {
            result: found_collections,
        }),
    ))
}

#[utoipa::path(get, path = "/v1/creators/{id}/uncollected", params(Pagination), responses((status = OK, body = SearchResult)), tags = ["creators"])]
pub async fn get_creators_uncollected(
    state: State<AppState>,
    Path(id): Path<i64>,
    pagination: Query<Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    let mut q = queries::base_search_query();
    q = media_by_creator(q, id);
    q = media_uncollected(q);
    let mut media_q = base_media();
    media_q = queries::pagination(media_q, pagination.0);
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

#[utoipa::path(get, path = "/v1/creators/{id}/media", params(Pagination), responses((status = OK, body = SearchResult)), tags = ["creators"])]
pub async fn get_creators_media(
    state: State<AppState>,
    Path(id): Path<i64>,
    pagination: Query<Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    let mut q = queries::base_search_query();
    q = media_by_creator(q, id);
    let mut media_q = base_media();
    media_q = media_from_search(media_q, q);
    media_q = queries::pagination(media_q, pagination.0);

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