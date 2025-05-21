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
use sea_orm::{
    ConnectionTrait, EntityTrait, FromQueryResult,
    QuerySelect, RelationTrait,
};
use sea_query::{Expr, JoinType};

#[utoipa::path(get, path = "/v1/creators", responses((status = OK, body = CreatorsResults)), tags = ["creators"])]
pub async fn get_creators(
    state: State<AppState>,
) -> Result<(StatusCode, Json<CreatorsResults>), AppError> {
    let creators = Creators::find()
        .join(JoinType::LeftJoin, creators::Relation::CreatorAlias.def())
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT creator_alias.alias) FILTER (WHERE creator_alias.creator = creators.id), '[]')"),
            "aliases")
        .group_by(creators::Column::Id)
        .into_model::<ApiCreator>()
        .all(&state.conn)
        .await?;
    Ok((StatusCode::OK, Json(CreatorsResults { result: creators })))
}

#[utoipa::path(get, path = "/v1/creators/{id}", responses((status = OK, body = ApiCreator)), tags = ["creators"])]
pub async fn get_creators_id(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiCreator>), AppError> {
    Ok((
           StatusCode::OK, Json(Creators::find_by_id(id)
        .join(JoinType::LeftJoin, creators::Relation::CreatorAlias.def())
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT creator_alias.alias) FILTER (WHERE creator_alias.creator = creators.id), '[]')"),
            "aliases")
        .group_by(creators::Column::Id)
        .into_model::<ApiCreator>()
        .one(&state.conn)
        .await?.expect("Creator not found"))
            ))
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


//
// #[utoipa::path(post, path = "/v1/collection", request_body = ApiCollection, responses((status = OK, body = ApiCollection)), tags = ["collection"])]
// pub async fn post_collection(
//     state: State<AppState>,
//     Json(payload): Json<ApiCollection>,
// ) -> Result<(StatusCode, Json<ApiCollection>), AppError> {
//     let txn: DatabaseTransaction = state.conn.begin().await?;
//     let new_model = Collections::insert(collections::ActiveModel {
//         name: Set(payload.name.unwrap()),
//         description: Set(payload.description),
//         ..Default::default()
//     })
//         .exec(&txn)
//         .await?;
//
//     if let Some(media) = payload.media {
//         for (i, m) in media.0.iter().enumerate() {
//             MediaCollection::insert(media_collection::ActiveModel {
//                 media_id: Set(*m),
//                 collection_id: Set(new_model.last_insert_id),
//                 ord: Set(i as i32),
//             })
//                 .exec(&txn)
//                 .await?;
//         }
//     }
//
//     txn.commit().await?;
//
//     let q = state
//         .conn
//         .get_database_backend()
//         .build(&queries::collection(new_model.last_insert_id));
//     let found_collection = ApiCollection::find_by_statement(q)
//         .one(&state.conn)
//         .await?
//         .unwrap();
//
//     Ok((StatusCode::OK, Json(found_collection)))
// }
//
// #[utoipa::path(get, path = "/v1/collection/{id}", responses((status = OK, body = ApiCollection)), tags = ["collection"])]
// pub async fn get_collection_id(
//     state: State<AppState>,
//     Path(id): Path<i64>,
// ) -> Result<(StatusCode, Json<ApiCollection>), AppError> {
//     let q = state
//         .conn
//         .get_database_backend()
//         .build(&queries::collection(id));
//     let found_collection = ApiCollection::find_by_statement(q).one(&state.conn).await?;
//     Ok((
//         StatusCode::OK,
//         Json(found_collection.expect("collection not found")),
//     ))
// }
//
// #[utoipa::path(patch, path = "/v1/collection/{id}", request_body = ApiCollection, responses((status = OK, body = ApiCollection)), tags = ["collection"])]
// pub async fn patch_collection_id(
//     state: State<AppState>,
//     Path(id): Path<i64>,
//     Json(payload): Json<ApiCollection>,
// ) -> Result<(StatusCode, Json<ApiCollection>), AppError> {
//     let q = state
//         .conn
//         .get_database_backend()
//         .build(&queries::collection(id));
//     let found_collection = ApiCollection::find_by_statement(q)
//         .one(&state.conn)
//         .await?
//         .expect("collection not found");
//     let txn: DatabaseTransaction = state.conn.begin().await?;
//     let new_model = Collections::update(collections::ActiveModel {
//         id: Set(id),
//         name: Set(payload.name.or(found_collection.name.clone()).unwrap()),
//         description: Set(payload.description.or(found_collection.description.clone())),
//         ..Default::default()
//     })
//         .exec(&txn)
//         .await?;
//
//     if let Some(media) = payload.media {
//         let current_hash: HashSet<i64> = found_collection.media.unwrap().0.into_iter().collect();
//         let new_hash: HashSet<i64> = media.clone().0.into_iter().collect();
//
//         let to_delete = current_hash.symmetric_difference(&new_hash);
//         for item in to_delete {
//             MediaCollection::delete_by_id((*item, id))
//                 .exec(&txn)
//                 .await?;
//         }
//
//         for (i, m) in media.0.iter().enumerate() {
//             match MediaCollection::find_by_id((*m, id)).one(&txn).await? {
//                 None => {
//                     MediaCollection::insert(media_collection::ActiveModel {
//                         media_id: Set(*m),
//                         collection_id: Set(id),
//                         ord: Set(i as i32),
//                     })
//                         .exec(&txn)
//                         .await?;
//                 }
//                 Some(entry) => {
//                     let mut active = entry.into_active_model();
//                     active.ord = Set(i as i32);
//                     MediaCollection::update(active).exec(&txn).await?;
//                 }
//             }
//         }
//     }
//
//     txn.commit().await?;
//
//     let q = state
//         .conn
//         .get_database_backend()
//         .build(&queries::collection(id));
//     let found_collection = ApiCollection::find_by_statement(q)
//         .one(&state.conn)
//         .await?
//         .unwrap();
//
//     Ok((StatusCode::OK, Json(found_collection)))
// }
//
// #[derive(utoipa::ToSchema, IntoParams, Debug, Deserialize, Clone)]
// pub struct AddItem {
//     media_id: i64,
//     ord: i32,
// }
//
// #[derive(utoipa::ToSchema, IntoParams, Debug, Deserialize)]
// pub struct AddQuery {
//     media: Vec<AddItem>,
// }
//
// #[utoipa::path(post, path = "/v1/collection/{id}/add", request_body = AddQuery, responses((status = OK)), tags = ["collection"])]
// pub async fn collection_id_add(
//     state: State<AppState>,
//     Path(id): Path<i64>,
//     Json(payload): Json<AddQuery>,
// ) -> Result<StatusCode, AppError> {
//     let entries: Vec<media_collection::ActiveModel> = payload
//         .media
//         .clone()
//         .into_iter()
//         .map(|(i)| media_collection::ActiveModel {
//             media_id: Set(i.media_id),
//             collection_id: Set(id),
//             ord: Set(i.ord),
//         })
//         .collect();
//
//     MediaCollection::insert_many(entries)
//         .exec_with_returning_many(&state.conn)
//         .await?;
//     Ok(StatusCode::CREATED)
// }
//
// // #[utoipa::path(post, path = "/v1/media", request_body(content = UploadForm, content_type = "multipart/form-data"),responses((status = OK, body = ApiMedia)), tags = ["media"])]
// // pub async fn post_collection() {
// //
// // }
