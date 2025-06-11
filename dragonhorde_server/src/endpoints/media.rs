use sea_orm::ColumnTrait;


use crate::api_models::{ApiMedia, DataMap, DataVector, Pagination, SearchResult};
use crate::endpoints::relations::collection_funcs::{collections_delete, collections_insert};
use crate::endpoints::relations::creator_funcs::{creator_delete, creators_insert};
use crate::endpoints::relations::source_funcs::{sources_delete, sources_insert};
use crate::error::AppError;
use crate::error::AppError::{BadRequest, Exists, NotFound};
use crate::{queries, AppState};
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use axum::Json;
use dragonhorde_common::hash::{perceptual, sha256};
use entity::{media, media::Entity as Media};
use image::ImageReader;
use image::imageops::Lanczos3;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    FromQueryResult, QueryFilter, Set, TransactionTrait,
};
use std::fs::File;
use std::io::{Cursor, Write};
use tokio::io::AsyncReadExt;
use utoipa::ToSchema;
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use crate::endpoints::relations::tag_funcs;

/// Check if the media item exists and return it as ApiMedia, raise a AppError:NotFound
async fn load_media_item(id: i64, db: &DatabaseConnection) -> Result<ApiMedia, AppError> {
    let q = db.get_database_backend().build(&queries::media_item(id));
    match ApiMedia::find_by_statement(q).one(db).await? {
        None => Err(NotFound(format!("media with id {} not found", id))),
        Some(m) => Ok(m),
    }
}

/// Check if the media item exists and return it as media::Model, raise a AppError:NotFound 
async fn load_media_item_model(id: i64, db: &DatabaseConnection) -> Result<media::Model, AppError> {
    match Media::find_by_id(id).one(db).await? {
        None => Err(NotFound(format!("media with id {} not found", id))),
        Some(m) => Ok(m),
    }
}

#[utoipa::path(get, path = "/v1/media", params(Pagination), responses((status = OK, body = SearchResult)), tags = ["media"])]
pub async fn get_media(
    state: State<AppState>,
    pagination: Query<Pagination>,
) -> Result<Json<SearchResult>, AppError> {
    let mut q = queries::base_media();
    q = queries::pagination(q, pagination.0);
    let statement = state.conn.get_database_backend().build(&q);
    let found_media = ApiMedia::find_by_statement(statement).all(&state.conn).await?;
    Ok(Json(SearchResult {result: found_media}))
}

async fn media_tag_update(
    tags: Option<DataMap>,
    new_model: &media::Model,
    db: &DatabaseTransaction,
    create: bool,
) -> Result<(), AppError> {
    if let Some(tag_groups) = tags {
        let tag_tuple = tag_funcs::groups_to_tuple(tag_groups.0);
        if !tag_tuple.is_empty() {
            let inserted_tags = tag_funcs::tags_insert(&tag_tuple, db).await?;
            tag_funcs::media_tags_insert(new_model.id, inserted_tags, db).await?;
        }
        if !create {
            tag_funcs::media_tags_delete(tag_tuple, &new_model, db).await?;
        }
    }
    Ok(())
}

async fn media_creators_update(
    creators: Option<DataVector>,
    new_model: &media::Model,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    if let Some(creators) = &creators {
        creators_insert(creators.0.clone(), new_model.id, db).await?;
        creator_delete(creators.0.clone(), new_model.id, db).await?;
    }
    Ok(())
}

async fn media_sources_update(
    sources: Option<DataVector>,
    new_model: &media::Model,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    if let Some(sources) = &sources {
        sources_insert(sources.0.clone(), new_model.id, db).await?;
        sources_delete(sources.0.clone(), new_model.id, db).await?;
    }
    Ok(())
}

#[derive(utoipa::ToSchema, Debug, TryFromMultipart)]
#[allow(unused)]
pub struct UploadForm {
    #[schema(value_type = ApiMedia)]
    data: String,
    #[schema(value_type = Vec<u8>, format = Binary, content_media_type = "application/octet-stream")]
    file: FieldData<Bytes>,
}

#[utoipa::path(post, path = "/v1/media", request_body(content = UploadForm, content_type = "multipart/form-data"),responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn post_media(
    state: State<AppState>,
    TypedMultipart(UploadForm { data, file }): TypedMultipart<UploadForm>,
) -> Result<Json<Option<ApiMedia>>, AppError> {
    let mut payload: ApiMedia = serde_json::from_str(data.as_str())?;

    let reader = ImageReader::new(Cursor::new(&file.contents)).with_guessed_format()?;
    let image_format = reader.format().ok_or(BadRequest("Can't Decode Image".to_string()))?;

    let im = reader.decode()?;
    if payload.perceptual_hash.is_none() {
        payload.perceptual_hash = Some(perceptual(&im))
    }

    let hash = sha256(&file.contents);

    if let Some(_) = Media::find()
        .filter(media::Column::Sha256.eq(&hash))
        .one(&state.conn)
        .await? {
        return Err(Exists(format!("media with sha256 {} already exists", &hash)));
    }

    let mut file_name: std::path::PathBuf = std::path::PathBuf::new();
    file_name.set_file_name(&hash);
    file_name.set_extension(&image_format.extensions_str()[0]);

    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_item: media::ActiveModel = media::ActiveModel {
        storage_uri: Set(file_name.to_string_lossy().to_string()),
        sha256: Set(hash),
        perceptual_hash: Set(payload.perceptual_hash.expect("perceptual_hash should be set")),
        created: Set(payload.created),
        title: Set(payload.title),
        r#type: Set(Some(image_format.extensions_str()[0].to_string())),
        description: Set(payload.description),
        ..Default::default()
    };

    let new_model = new_item.insert(&txn).await?;

    media_tag_update(payload.tag_groups, &new_model, &txn, true).await?;
    media_creators_update(payload.creators, &new_model, &txn).await?;
    media_sources_update(payload.sources, &new_model, &txn).await?;
    if let Some(collections) = payload.collections {
        collections_insert(&collections.0, new_model.id, &txn).await?;
    }
    let media_path = state.storage_dir.clone().join(&file_name);
    let mut media_file = File::create(&media_path)?;
    media_file.write_all(&file.contents)?;

    let mut thumbnail_path = state.thumbnail_dir.clone();
    thumbnail_path.push(new_model.sha256);
    thumbnail_path.set_extension("webp");
    let thumbnail = im.resize(400, 400, Lanczos3);
    thumbnail.save(&thumbnail_path)?;

    //End of Transaction
    match txn.commit().await {
        Ok(_) => {
            Ok(Json(Some(load_media_item(new_model.id, &state.conn).await?)))
        }
        Err(e) => {
            //If the transaction fails, remove the files
            std::fs::remove_file(media_path).ok();
            std::fs::remove_file(thumbnail_path).ok();
            Err(AppError::from(e))
        }
    }
}

#[utoipa::path(post, path = "/v1/media/{id}", request_body = ApiMedia , responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn update_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiMedia>,
) -> Result<Json<ApiMedia>, AppError> {
    let current = load_media_item_model(id, &state.conn).await?;
    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_model = Media::update(media::ActiveModel {
        id: Set(id),
        perceptual_hash: Set(payload.perceptual_hash.expect("REASON")),
        created: Set(payload.created),
        title: Set(payload.title),
        description: Set(payload.description),
        ..Default::default()
    })
    .exec(&txn)
    .await?;

    media_tag_update(payload.tag_groups, &new_model, &txn, false).await?;
    media_creators_update(payload.creators, &new_model, &txn).await?;
    media_sources_update(payload.sources, &new_model, &txn).await?;

    if let Some(collections) = payload.collections {
        collections_insert(&collections.0, new_model.id, &txn).await?;
        collections_delete(&collections.0, new_model.id, &txn).await?;
    }

    txn.commit().await?;
    //End of Transaction

    Ok(Json(load_media_item(id, &state.conn).await?))
}

#[utoipa::path(patch, path = "/v1/media/{id}", request_body = ApiMedia , responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn media_item_patch(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiMedia>,
) -> Result<(), AppError> {
    let item = load_media_item(id, &state.conn).await?;

    let txn: DatabaseTransaction = state.conn.begin().await?;
    let new_model = Media::update(media::ActiveModel {
        id: Set(id),
        perceptual_hash: Set(payload.perceptual_hash.unwrap_or(item.perceptual_hash.clone().unwrap())),
        created: Set(payload.created.or(item.created.clone())),
        title: Set(payload.title.or(item.title.clone())),
        description: Set(payload.description.or(item.description.clone())),
        ..Default::default()
    })
    .exec(&txn)
    .await?;

    media_tag_update(payload.tag_groups, &new_model, &txn, false).await?;
    media_creators_update(payload.creators, &new_model, &txn).await?;
    media_sources_update(payload.sources, &new_model, &txn).await?;

    if let Some(collections) = payload.collections {
        collections_insert(&collections.0, new_model.id, &txn).await?;
        collections_delete(&collections.0, new_model.id, &txn).await?;
    }

    txn.commit().await?;
    //End of Transaction

    Ok(())
}

#[utoipa::path(get, path = "/v1/media/{id}", responses((status = OK, body = ApiMedia)), tags = ["media"]
)]
pub async fn get_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMedia>, AppError> {
    Ok(Json(load_media_item(id, &state.conn).await?))
}

#[utoipa::path(get, path = "/v1/media/by_hash/{hash}", responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn get_media_item_by_hash(
    state: State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiMedia>, AppError> {
    let mut q = queries::base_media();
    q = queries::media_by_sha(q, &id);

    let statement = state.conn.get_database_backend().build(&q);
    match ApiMedia::find_by_statement(statement)
        .one(&state.conn)
        .await?
    {
        None => Err(NotFound(format!("media with id {} not found", id))),
        Some(m) => Ok(Json(m)),
    }
}

fn extension_to_mime(ext: &str) -> &'static str {
    match ext {
        "apng" => "image/apng",
        "avif" => "image/avif",
        "gif" => "image/gif",
        "jpg" => "image/jpeg",
        "jpeg" => "image/jpeg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[expect(unused)]
pub struct Binary(String);

#[utoipa::path(get, path = "/v1/media/{id}/file", responses((status = OK, body = Binary, content_type = "application/octet")), tags = ["media"])]
pub async fn get_media_file(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let media_item = load_media_item_model(id, &state.conn).await?;

    let path = &state.storage_dir.join(media_item.storage_uri.clone());
    let mut file = tokio::fs::File::open(path).await?;
    let mut data: Vec<u8> = Vec::new();
    file.read_to_end(&mut data).await?;

    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        extension_to_mime(&media_item.r#type.expect("media type missing")).parse()?,
    );
    if let Some(media_path) = path.file_name() {
        headers.append(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", media_path.to_string_lossy()).parse()?,
        );
    }

    Ok((headers, Body::from(data)).into_response())
}

#[utoipa::path(get, path = "/v1/media/{id}/thumbnail",responses((status = OK, body = Binary, content_type = "application/octet")), tags = ["media"])]
pub async fn get_media_thumbnail(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let media_item = load_media_item_model(id, &state.conn).await?;

    let mut data: Vec<u8> = Vec::new();

    let thumbnail_path = state
        .thumbnail_dir
        .join(format!("{}.webp", &media_item.sha256));
    if thumbnail_path.exists() {
        let mut file = tokio::fs::File::open(&thumbnail_path).await?;
        file.read_to_end(&mut data).await?;
    } else {
        let file_path = &state.storage_dir.join(media_item.storage_uri.clone());
        let reader = ImageReader::open(file_path)?;
        let im = reader.decode()?;
        let thumbnail = im.resize(400, 400, Lanczos3);
        thumbnail.save(&thumbnail_path)?;
        thumbnail.write_to(&mut Cursor::new(&mut data), image::ImageFormat::WebP)?;
    }

    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/webp".parse()?);
    if let Some(thumbnail_file) = thumbnail_path.file_name() {
        headers.append(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}\"",
                thumbnail_file.to_string_lossy()
            )
            .parse()?,
        );
    }

    Ok((headers, Body::from(data)).into_response())
}
