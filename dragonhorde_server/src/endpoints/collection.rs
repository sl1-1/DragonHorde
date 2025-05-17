use std::collections::HashSet;
use crate::endpoints::media::{DataMap, DataVector, Pagination};
use crate::error::AppError;
use crate::{AppState, queries};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset};
use entity::{collections, collections::Entity as Collections};
use entity::{media_collection, media_collection::Entity as MediaCollection};
use sea_orm::{ConnectionTrait, DatabaseTransaction, EntityTrait, FromJsonQueryResult, FromQueryResult, IntoActiveModel, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use entity::media_collection::Model;

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, FromJsonQueryResult,
)]
pub struct DataVectorI64(pub Vec<i64>);
impl Default for DataVectorI64 {
    fn default() -> Self {
        Self(Vec::new())
    }
}

#[serde_with::skip_serializing_none]
#[derive(
    utoipa::ToSchema,
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    FromQueryResult,
    FromJsonQueryResult,
)]
pub struct ApiCollection {
    #[schema(read_only, value_type = i64)]
    pub id: Option<i64>,
    /// date-time that this item was created, if known
    pub created: Option<DateTime<FixedOffset>>,
    pub name: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub creators: Option<DataVector>,
    #[serde(default)]
    #[schema(value_type = Option<BTreeMap<String, Vec<String>>>)]
    pub tag_groups: Option<DataMap>,
    /// Description of this item, if available
    pub description: Option<String>,
    #[schema(value_type = Option<Vec<i64>>)]
    #[serde(default)]
    pub media: Option<DataVectorI64>,
}

#[serde_with::skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionResult {
    pub result: Vec<ApiCollection>,
}

#[utoipa::path(get, path = "/v1/collection", params(Pagination), responses((status = OK, body = CollectionResult)), tags = ["collection"])]
pub async fn get_collections(
    state: State<AppState>,
    pagination: Query<Pagination>,
) -> Result<(StatusCode, Json<CollectionResult>), AppError> {
    let mut q = queries::base_collection();
    // q = queries::pagination(q, pagination.0);
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

#[utoipa::path(get, path = "/v1/collection/{id}", responses((status = OK, body = ApiCollection)), tags = ["collection"])]
pub async fn get_collection_id(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiCollection>), AppError> {
    let q = state
        .conn
        .get_database_backend()
        .build(&queries::collection(id));
    let found_collection = ApiCollection::find_by_statement(q).one(&state.conn).await?;
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
    let q = state
        .conn
        .get_database_backend()
        .build(&queries::collection(id));
    let found_collection = ApiCollection::find_by_statement(q)
        .one(&state.conn)
        .await?
        .expect("collection not found");
    let txn: DatabaseTransaction = state.conn.begin().await?;
    let new_model = Collections::update(collections::ActiveModel {
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
            MediaCollection::delete_by_id((*item, id)).exec(&txn).await?;
        }
        
        for (i, m) in media.0.iter().enumerate() {
            match MediaCollection::find_by_id((*m, id)).one(&txn).await? {
                None => {
                    MediaCollection::insert(media_collection::ActiveModel {
                        media_id: Set(*m),
                        collection_id: Set(id),
                        ord: Set(i as i32),
                    }).exec(&txn).await?;
                }
                Some(entry) => {
                    let mut active = entry.into_active_model();
                    active.ord = Set(i as i32);
                    MediaCollection::update(active).exec(&txn).await?;
                }
            }
        }

    }

    txn.commit().await?;

    let q = state
        .conn
        .get_database_backend()
        .build(&queries::collection(id));
    let found_collection = ApiCollection::find_by_statement(q)
        .one(&state.conn)
        .await?
        .unwrap();

    Ok((StatusCode::OK, Json(found_collection)))
}

// #[utoipa::path(post, path = "/v1/media", request_body(content = UploadForm, content_type = "multipart/form-data"),responses((status = OK, body = ApiMedia)), tags = ["media"])]
// pub async fn post_collection() {
//
// }
