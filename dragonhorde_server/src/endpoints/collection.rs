use crate::api_models::ApiCollection;
use crate::api_models::Pagination;
use crate::error::AppError;
use crate::{queries, AppState};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use entity::{collections, collections::Entity as Collections};
use entity::{media_collection, media_collection::Entity as MediaCollection};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, EntityTrait, FromQueryResult,
    IntoActiveModel, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::IntoParams;

#[serde_with::skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionResult {
    pub result: Vec<ApiCollection>,
}

#[utoipa::path(get, path = "/v1/collection", responses((status = OK, body = CollectionResult)), tags = ["collection"])]
pub async fn get_collections(
    state: State<AppState>,
) -> Result<(StatusCode, Json<CollectionResult>), AppError> {
    let q = queries::base_collection();
    let statement = state.conn.get_database_backend().build(&q);
    let found_collections = ApiCollection::find_by_statement(statement)
        .all(&state.conn)
        .await?;
    dbg!(&found_collections);
    Ok((
        StatusCode::OK,
        Json(CollectionResult {
            result: found_collections,
        }),
    ))
}

#[utoipa::path(post, path = "/v1/collection", request_body = ApiCollection, responses((status = OK, body = ApiCollection)), tags = ["collection"])]
pub async fn post_collection(
    state: State<AppState>,
    Json(payload): Json<ApiCollection>,
) -> Result<(StatusCode, Json<ApiCollection>), AppError> {
    let txn: DatabaseTransaction = state.conn.begin().await?;
    let new_model = Collections::insert(collections::ActiveModel {
        name: Set(payload.name.unwrap()),
        description: Set(payload.description),
        ..Default::default()
    })
    .exec(&txn)
    .await?;

    if let Some(media) = payload.media {
        for (i, m) in media.0.iter().enumerate() {
            MediaCollection::insert(media_collection::ActiveModel {
                media_id: Set(*m),
                collection_id: Set(new_model.last_insert_id),
                ord: Set(Some(i as i32)),
            })
            .exec(&txn)
            .await?;
        }
    }

    txn.commit().await?;

    let mut q = queries::base_collection();
    q = queries::collection(q, new_model.last_insert_id);
    q = queries::collection_with_media(q);
    let statement = state
        .conn
        .get_database_backend()
        .build(&q);
    let found_collection = ApiCollection::find_by_statement(statement)
        .one(&state.conn)
        .await?
        .unwrap();

    Ok((StatusCode::OK, Json(found_collection)))
}

#[utoipa::path(get, path = "/v1/collection/{id}", responses((status = OK, body = ApiCollection)), tags = ["collection"])]
pub async fn get_collection_id(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiCollection>), AppError> {
    let mut q = queries::base_collection();
    q = queries::collection(q, id);
    q = queries::collection_with_media(q);
        let statement = state
        .conn
        .get_database_backend()
        .build(&q);
    let found_collection = ApiCollection::find_by_statement(statement).one(&state.conn).await?;
    Ok((
        StatusCode::OK,
        Json(found_collection.expect("collection not found")),
    ))
}

#[utoipa::path(patch, path = "/v1/collection/{id}", request_body = ApiCollection, responses((status = OK, body = ApiCollection)), tags = ["collection"])]
pub async fn patch_collection_id(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiCollection>,
) -> Result<(StatusCode, Json<ApiCollection>), AppError> {
    let mut q = queries::base_collection();
    q = queries::collection(q, id);
    q = queries::collection_with_media(q);
    let statement = state
        .conn
        .get_database_backend()
        .build(&q);
    let found_collection = ApiCollection::find_by_statement(statement)
        .one(&state.conn)
        .await?
        .expect("collection not found");
    let txn: DatabaseTransaction = state.conn.begin().await?;
    Collections::update(collections::ActiveModel {
        id: Set(id),
        name: Set(payload.name.or(found_collection.name.clone()).unwrap()),
        description: Set(payload.description.or(found_collection.description.clone())),
        ..Default::default()
    })
    .exec(&txn)
    .await?;

    if let Some(media) = payload.media {
        let current_hash: HashSet<i64> = found_collection.media.unwrap().0.into_iter().collect();
        let new_hash: HashSet<i64> = media.clone().0.into_iter().collect();

        let to_delete = current_hash.symmetric_difference(&new_hash);
        for item in to_delete {
            MediaCollection::delete_by_id((*item, id))
                .exec(&txn)
                .await?;
        }

        for (i, m) in media.0.iter().enumerate() {
            match MediaCollection::find_by_id((*m, id)).one(&txn).await? {
                None => {
                    MediaCollection::insert(media_collection::ActiveModel {
                        media_id: Set(*m),
                        collection_id: Set(id),
                        ord: Set(Some(i as i32)),
                    })
                    .exec(&txn)
                    .await?;
                }
                Some(entry) => {
                    let mut active = entry.into_active_model();
                    active.ord = Set(Some(i as i32));
                    MediaCollection::update(active).exec(&txn).await?;
                }
            }
        }
    }

    txn.commit().await?;

    let mut q = queries::base_collection();
    q = queries::collection(q, id);
    q = queries::collection_with_media(q);
    let statement = state
        .conn
        .get_database_backend()
        .build(&q);
    let found_collection = ApiCollection::find_by_statement(statement)
        .one(&state.conn)
        .await?
        .unwrap();

    Ok((StatusCode::OK, Json(found_collection)))
}

#[derive(utoipa::ToSchema, IntoParams, Debug, Deserialize, Clone)]
pub struct AddItem {
    media_id: i64,
    ord: i32,
}

#[derive(utoipa::ToSchema, IntoParams, Debug, Deserialize)]
pub struct AddQuery {
    media: Vec<AddItem>,
}

#[utoipa::path(post, path = "/v1/collection/{id}/add", request_body = AddQuery, responses((status = OK)), tags = ["collection"])]
pub async fn collection_id_add(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<AddQuery>,
) -> Result<StatusCode, AppError> {
    let entries: Vec<media_collection::ActiveModel> = payload
        .media
        .clone()
        .into_iter()
        .map(|(i)| media_collection::ActiveModel {
            media_id: Set(i.media_id),
            collection_id: Set(id),
            ord: Set(Some(i.ord)),
        })
        .collect();

    MediaCollection::insert_many(entries)
        .exec_with_returning_many(&state.conn)
        .await?;
    Ok(StatusCode::CREATED)
}

// #[utoipa::path(post, path = "/v1/media", request_body(content = UploadForm, content_type = "multipart/form-data"),responses((status = OK, body = ApiMedia)), tags = ["media"])]
// pub async fn post_collection() {
//
// }
