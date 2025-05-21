use sea_orm::ColumnTrait;
mod collection_funcs;
mod creator_funcs;
mod source_funcs;
mod tag_funcs;

use crate::api_models::{ApiMedia, DataMap, DataVector, Pagination, SearchResult};
use crate::endpoints::media::collection_funcs::{collections_delete, collections_insert};
use crate::endpoints::media::creator_funcs::{creator_delete, creators_insert};
use crate::endpoints::media::source_funcs::{sources_delete, sources_insert};
use crate::error::AppError;
use crate::{queries, AppState};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{extract, Json};
use dragonhorde_common::hash::{perceptual, sha256};
use entity::{media, media::Entity as Media};
use image::imageops::Lanczos3;
use image::ImageReader;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    FromQueryResult, QueryFilter, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Cursor, Write};
use tokio::io::AsyncReadExt;
use utoipa::ToSchema;

async fn load_media_item(id: i64, db: &DatabaseConnection) -> Result<ApiMedia, AppError> {
    let q = db.get_database_backend().build(&queries::media_item(id));
    match ApiMedia::find_by_statement(q).one(db).await? {
        None => Err(AppError::NotFound(format!(
            "media with id {} not found",
            id
        ))),
        Some(m) => Ok(m),
    }
}

async fn check_media_exists(id: i64, db: &DatabaseConnection) -> Result<media::Model, AppError> {
    match Media::find_by_id(id).one(db).await? {
        None => Err(AppError::NotFound(format!(
            "media with id {} not found",
            id
        ))),
        Some(m) => Ok(m),
    }
}

#[utoipa::path(get, path = "/v1/media", params(Pagination), responses((status = OK, body = SearchResult)), tags = ["media"])]
pub async fn get_media(
    state: State<AppState>,
    pagination: Query<Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    let mut q = queries::base_media();
    q = queries::pagination(q, pagination.0);
    let statement = state.conn.get_database_backend().build(&q);
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

async fn media_tag_update(
    tags: Option<DataMap>,
    new_model: &media::Model,
    db: &DatabaseTransaction,
    create: bool,
) -> Result<(), AppError> {
    if let Some(tag_groups) = &tags {
        let tag_tuple = tag_funcs::groups_to_tuple(&tag_groups.0);
        if !tag_tuple.is_empty() {
            let new_groups = tag_funcs::tag_group_insert(&tag_tuple, db).await?;
            let inserted_tags = tag_funcs::tags_insert(&tag_tuple, &new_groups, db).await?;
            tag_funcs::tags_insert_relations(new_model.id, inserted_tags, db).await?;
        }
        if !create {
            tag_funcs::tags_update(tag_tuple, &new_model, db).await?;
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

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(unused)]
struct UploadForm {
    data: ApiMedia,
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    file: String,
}

#[utoipa::path(post, path = "/v1/media", request_body(content = UploadForm, content_type = "multipart/form-data"),responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn post_media(
    state: State<AppState>,
    mut multipart: extract::Multipart,
) -> Result<(StatusCode, Json<Option<ApiMedia>>), AppError> {
    let mut payload_option: Option<ApiMedia> = None;
    let mut file_option: Option<Vec<u8>> = None;

    //Unpack the multipart form into the metadata and file
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap().to_string();
        println!("name: {}", name);
        if name == "data" {
            let text = field.text().await?;
            payload_option = Some(serde_json::from_str(text.as_str())?);
        } else if name == "file" {
            file_option = Some(Vec::from(field.bytes().await?));
        }
    }

    let mut payload = match payload_option {
        Some(payload) => payload,
        None => return Ok((StatusCode::BAD_REQUEST, Json(None))),
    };

    let file = match file_option {
        Some(file) => file,
        None => return Ok((StatusCode::BAD_REQUEST, Json(None))),
    };

    let reader = ImageReader::new(Cursor::new(&file)).with_guessed_format()?;
    let image_format = reader
        .format()
        .expect("Image Format")
        .extensions_str()
        .first()
        .unwrap()
        .to_string();
    let im = reader.decode()?;
    if payload.perceptual_hash.is_none() {
        payload.perceptual_hash = Some(perceptual(&im))
    }

    let hash = sha256(&file);

    match Media::find()
        .filter(media::Column::Sha256.eq(&hash))
        .one(&state.conn)
        .await?
    {
        None => {}
        Some(_) => {
            return Err(AppError::Exists(format!(
                "media with sha256 {} already exists",
                &hash
            )));
        }
    }

    let mut file_name: std::path::PathBuf = std::path::PathBuf::new();
    file_name.set_file_name(&hash);
    file_name.set_extension(&image_format);

    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_item: media::ActiveModel = media::ActiveModel {
        storage_uri: Set(file_name.to_string_lossy().to_string()),
        sha256: Set(hash),
        perceptual_hash: Set(payload.perceptual_hash),
        created: Set(payload.created),
        title: Set(payload.title),
        r#type: Set(Some(image_format)),
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
    let mut out = File::create(state.storage_dir.clone().join(file_name))?;
    out.write_all(&file)?;

    let mut path = state.thumbnail_dir.clone();
    path.push(new_model.sha256);
    path.set_extension("webp");
    let thumbnail = im.resize(400, 400, Lanczos3);
    thumbnail.save(path)?;

    txn.commit().await?;
    //End of Transaction

    Ok((
        StatusCode::OK,
        Json(Some(load_media_item(new_model.id, &state.conn).await?)),
    ))
}

#[utoipa::path(post, path = "/v1/media/{id}", request_body = ApiMedia , responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn update_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiMedia>,
) -> Result<(StatusCode, Json<ApiMedia>), AppError> {
    check_media_exists(id, &state.conn).await?;
    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_model = Media::update(media::ActiveModel {
        id: Set(id),
        perceptual_hash: Set(payload.perceptual_hash),
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

    Ok((
        StatusCode::OK,
        Json(load_media_item(id, &state.conn).await?),
    ))
}

#[utoipa::path(patch, path = "/v1/media/{id}", request_body = ApiMedia , responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn media_item_patch(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiMedia>,
) -> Result<StatusCode, AppError> {
    let item = load_media_item(id, &state.conn).await?;

    let txn: DatabaseTransaction = state.conn.begin().await?;
    let new_model = Media::update(media::ActiveModel {
        id: Set(id),
        perceptual_hash: Set(payload.perceptual_hash.or(item.perceptual_hash.clone())),
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

    txn.commit().await?;
    //End of Transaction

    Ok(StatusCode::OK)
}

#[utoipa::path(get, path = "/v1/media/{id}", responses((status = OK, body = ApiMedia)), tags = ["media"]
)]
pub async fn get_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiMedia>), AppError> {
    Ok((
        StatusCode::OK,
        Json(load_media_item(id, &state.conn).await?),
    ))
}

#[utoipa::path(get, path = "/v1/media/by_hash/{hash}", responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn get_media_item_by_hash(
    state: State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<ApiMedia>), AppError> {
    let mut q = queries::base_media();
    q = queries::media_by_sha(q, &id);

    let statement = state.conn.get_database_backend().build(&q);
    let found_media = ApiMedia::find_by_statement(statement)
        .one(&state.conn)
        .await?;
    match found_media {
        None => Err(AppError::NotFound(format!(
            "media with id {} not found",
            id
        ))),
        Some(m) => Ok((StatusCode::OK, Json(m))),
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
pub struct Binary(String);

#[utoipa::path(get, path = "/v1/media/{id}/file", responses((status = OK, body = Binary, content_type = "application/octet")), tags = ["media"])]
pub async fn get_media_file(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Response), AppError> {
    let media_item = check_media_exists(id, &state.conn).await?;

    let path = &state
        .storage_dir
        .clone()
        .join(media_item.storage_uri.clone());
    println!("path: {:?}", path);
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Ok((
                StatusCode::NOT_FOUND,
                format!("Error Reading {:?} {:?}", path, err).into_response(),
            ));
        }
    };
    let mut data: Vec<u8> = Vec::new();
    file.read_to_end(&mut data).await?;

    let headers = [
        (
            header::CONTENT_TYPE,
            extension_to_mime(&path.extension().unwrap().to_string_lossy()),
        ),
        (
            header::CONTENT_DISPOSITION,
            &format!("attachment; filename=\"{}\"", media_item.storage_uri),
        ),
    ];
    Ok((StatusCode::OK, (headers, Body::from(data)).into_response()))
}

#[utoipa::path(get, path = "/v1/media/{id}/thumbnail",responses((status = OK, body = Binary, content_type = "application/octet")), tags = ["media"])]
pub async fn get_media_thumbnail(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Response), AppError> {
    let media_item = check_media_exists(id, &state.conn).await?;

    let mut data: Vec<u8> = Vec::new();

    let mut path = state.thumbnail_dir.clone();
    path.push(&media_item.sha256);
    path.set_extension("webp");
    if path.exists() {
        let mut file = match tokio::fs::File::open(&path).await {
            Ok(file) => file,
            Err(err) => {
                return Ok((
                    StatusCode::NOT_FOUND,
                    format!("Error Reading {:?} {:?}", path, err).into_response(),
                ));
            }
        };
        file.read_to_end(&mut data).await?;
    } else {
        let mut file_path = state.storage_dir.clone();
        file_path.push(&media_item.sha256);
        file_path.set_extension(media_item.r#type.expect("File Extension"));
        let reader = ImageReader::open(file_path)?;
        let im = reader.decode()?;
        let thumbnail = im.resize(400, 400, Lanczos3);
        thumbnail.save(&path)?;
        thumbnail.write_to(&mut Cursor::new(&mut data), image::ImageFormat::WebP)?;
    }

    let headers = [
        (header::CONTENT_TYPE, "image/webp"),
        (
            header::CONTENT_DISPOSITION,
            &format!(
                "attachment; filename=\"{}\"",
                path.file_name().unwrap().to_string_lossy()
            ),
        ),
    ];
    Ok((StatusCode::OK, (headers, Body::from(data)).into_response()))
}
