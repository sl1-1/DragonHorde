use crate::api_models::{ApiCollection, ApiCollectionResult};
use crate::endpoints::media::Binary;
use crate::endpoints::shared::creators_create;
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::FixedOffset;
use std::collections::{HashMap, HashSet};
use utoipa::IntoParams;

#[serde_with::skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionResult {
    pub result: Vec<ApiCollectionResult>,
}

#[utoipa::path(get, path = "/v1/collection", responses((status = OK, body = CollectionResult)), tags = ["collection"])]
pub async fn get_collections(
    state: State<AppState>,
) -> Result<(StatusCode, Json<CollectionResult>), AppError> {
    let found_collections = sqlx::query_file_as!(
        ApiCollectionResult,
        "sql/endpoints/collections/get_collections.sqlx",
        false,
        &vec![][..]
    )
    .fetch_all(&state.conn)
    .await?;
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
) -> Result<Json<ApiCollectionResult>, AppError> {
    if let Some(name) = payload.name {
        //Check if already exists
        if sqlx::query_file_scalar!(
            "sql/endpoints/collections/get_collection_by_path.sqlx",
            &name
                .split("/")
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
        )
        .fetch_optional(&state.conn)
        .await?
        .is_some()
        {
            return Err(AppError::BadRequest(format!(
                "Collection {} Already Exists",
                name
            )));
        }

        let mut tx = state.conn.begin().await?;
        let path: Vec<String> = name.split("/").map(|i| i.to_string()).collect();
        dbg!(&path);
        let mut parent: Option<i64> = None;

        //Recursively create collection
        for (pos, part) in path.iter().enumerate() {
            dbg!(&path[0..pos + 1]);
            match sqlx::query_file_scalar!(
                "sql/endpoints/collections/get_collection_by_path.sqlx",
                &path[0..pos + 1]
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                None => {
                    let mut description: Option<String> = None;
                    if pos == path.len() {
                        description = payload.description.clone();
                    }
                    parent = Some(
                        sqlx::query_scalar!(
                            r#"INSERT INTO collections(name, parent, description)
                                VALUES ($1, $2::bigint, $3) RETURNING id"#,
                            part,
                            parent,
                            description
                        )
                        .fetch_one(&mut *tx)
                        .await?,
                    );
                }
                Some(r) => {
                    parent = Some(r);
                    dbg!(&r);
                }
            }
        }

        if let Some(media) = payload.media {
            let (order, media): (Vec<usize>, Vec<i64>) = media.0.iter().enumerate().collect();
            sqlx::query!(
                r#"
                    INSERT INTO media_collection(collection_id, media_id, ord)
                    SELECT $1, * FROM unnest($2::bigint[], $3::int[])
                "#,
                parent,
                &media[..],
                &order.into_iter().map(|i| i as i32).collect::<Vec<i32>>()[..]
            )
            .execute(&mut *tx)
            .await?;
        }

        if let Some(creators) = payload.creators {
            let mut creators_in = creators.0;
            creators_in.sort_by_key(|c| c.to_lowercase());
            creators_in.dedup_by_key(|c| c.to_lowercase());
            if !creators_in.is_empty() {
                let mut creators_inserted: Vec<i64> = creators_create(creators_in, &mut tx).await?;
                sqlx::query!(
                    r#"
                                INSERT INTO collection_creators(collection_id, creator_id)
                                SELECT $1, * FROM unnest($2::bigint[])"#,
                    parent,
                    &creators_inserted[..]
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        let r = sqlx::query_file_as!(
            ApiCollectionResult,
            "sql/endpoints/collections/get_collections.sqlx",
            true,
            &vec![parent.unwrap()][..]
        )
        .fetch_one(&state.conn)
        .await?;

        Ok(Json(r))
    } else {
        Err(AppError::BadRequest("name required".to_string()))
    }
}

#[utoipa::path(get, path = "/v1/collection/{id}", responses((status = OK, body = ApiCollection)), tags = ["collection"])]
pub async fn get_collection_id(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiCollectionResult>, AppError> {
    match sqlx::query_file_as!(
        ApiCollectionResult,
        "sql/endpoints/collections/get_collections.sqlx",
        true,
        &vec![id][..]
    )
    .fetch_optional(&state.conn)
    .await?
    {
        None => Err(NotFound(format!("Collection {} not found", id))),
        Some(c) => Ok(Json(c)),
    }
}

#[utoipa::path(get, path = "/v1/collection/by_path/{*path}", responses((status = OK, body = ApiCollection)), tags = ["collection"])]
pub async fn get_collection_path(
    state: State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<ApiCollectionResult>, AppError> {
    match sqlx::query_file_scalar!(
        "sql/endpoints/collections/get_collection_by_path.sqlx",
        &path
            .split("/")
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
    )
    .fetch_optional(&state.conn)
    .await?
    {
        None => Err(NotFound(format!("Collection {} not found", path))),
        Some(c) => Ok(Json(
            sqlx::query_file_as!(
                ApiCollectionResult,
                "sql/endpoints/collections/get_collections.sqlx",
                true,
                &vec![c][..]
            )
            .fetch_one(&state.conn)
            .await?,
        )),
    }
}

#[utoipa::path(patch, path = "/v1/collection/{id}", request_body = ApiCollection, responses((status = OK, body = ApiCollection)), tags = ["collection"])]
pub async fn patch_collection_id(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiCollection>,
) -> Result<Json<ApiCollectionResult>, AppError> {
    let r = match sqlx::query_file_as!(
        ApiCollectionResult,
        "sql/endpoints/collections/get_collections.sqlx",
        true,
        &vec![id][..]
    )
    .fetch_optional(&state.conn)
    .await?
    {
        None => return Err(NotFound(format!("Collection {} not found", id))),
        Some(c) => c,
    };

    let mut tx = state.conn.begin().await?;

    let name = payload.name.unwrap_or(r.name.unwrap()).split("/").last().unwrap().to_string();

    sqlx::query!(
        r#"UPDATE collections SET name=$2, description=$3 WHERE id=$1"#,
        id,
        name,
        payload.description.or(r.description)
    )
    .execute(&mut *tx)
    .await?;

    if let Some(media) = payload.media {
        let current_hash: HashSet<i64> = r.media.unwrap().into_iter().collect();
        let new_hash: HashSet<i64> = media.clone().0.into_iter().collect();

        let to_delete = current_hash.symmetric_difference(&new_hash);
        sqlx::query!("DELETE FROM media_collection WHERE collection_id=$1", id)
            .execute(&mut *tx)
            .await?;

        let (order, media): (Vec<usize>, Vec<i64>) = media.0.iter().enumerate().collect();

        sqlx::query!(
            "INSERT INTO media_collection(collection_id, media_id, ord)
                    SELECT $1, * FROM unnest($2::bigint[], $3::int[])",
            id,
            &media[..],
            &order.into_iter().map(|i| i as i32).collect::<Vec<i32>>()[..]
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(creators) = payload.creators {
        let mut creators_in = creators.0;
        creators_in.sort_by_key(|c| c.to_lowercase());
        creators_in.dedup_by_key(|c| c.to_lowercase());
        if !creators_in.is_empty() {
            let mut creators_inserted: Vec<i64> = creators_create(creators_in.clone(), &mut tx).await?;
            sqlx::query!(
                r#"
                                INSERT INTO collection_creators(collection_id, creator_id)
                                SELECT $1, * FROM unnest($2::bigint[]) ON CONFLICT DO NOTHING "#,
                id,
                &creators_inserted[..]
            )
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query!(
            r#"
            DELETE
            FROM collection_creators
            WHERE collection_id = $1
              AND NOT creator_id <> ALL (
                SELECT collection_creators.creator_id
                FROM collection_creators
                         LEFT JOIN creator_alias ON creator_alias.creator = creator_id
                WHERE collection_id = $1
                  AND alias <> ALL ($2::varchar[])
                  )
            "#,
            id,
            &creators_in[..]
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    let r = sqlx::query_file_as!(
            ApiCollectionResult,
            "sql/endpoints/collections/get_collections.sqlx",
            true,
            &vec![id][..]
        )
        .fetch_one(&state.conn)
        .await?;

    Ok(Json(r))
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
    let (media, order): (Vec<i64>, Vec<i32>) = payload
        .media
        .clone()
        .into_iter()
        .map(|i| (i.media_id, i.ord))
        .collect();

    sqlx::query!(r#"INSERT INTO media_collection(collection_id, media_id, ord) SELECT $1, * FROM unnest($2::bigint[], $3::int[])"#,
    id, &media[..], &order[..]
    ).execute(&state.conn).await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(get, path = "/v1/collection/{id}/thumbnail", responses((status = OK, body = Binary, content_type = "application/octet")), tags = ["collection"])]
pub async fn get_collection_id_thumbnail(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Redirect, AppError> {
    match sqlx::query_scalar!(
        r#"SELECT media_collection.media_id FROM media_collection WHERE media_collection.collection_id = $1 ORDER BY media_collection.ord ASC"#,
        id
    )
        .fetch_optional(&state.conn)
        .await? {
        None => Err(NotFound(format!("Collection {} not found", id))),
        Some(m) => {Ok(Redirect::permanent(&format!("/v1/media/{}/thumbnail", m)))}
    }
}
