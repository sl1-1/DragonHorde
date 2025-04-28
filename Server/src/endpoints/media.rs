mod creator_funcs;
mod source_funcs;
mod tag_funcs;

pub use tag_funcs::{media_add_tag, media_delete_tag, media_get_tags};

use crate::endpoints::media::creator_funcs::{creator_delete, creators_insert};
use crate::endpoints::media::source_funcs::{sources_delete, sources_insert};
use crate::error::AppError;
use crate::{AppState, queries};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, extract};
use entity::{media, media::Entity as Media};
use image::ImageReader;
use image::imageops::Lanczos3;
use img_hash::HashAlg::Gradient;
use img_hash::HasherConfig;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    FromJsonQueryResult, FromQueryResult, PaginatorTrait, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Cursor, Write};
use tokio::io::AsyncReadExt;

#[derive(Deserialize)]
pub struct Pagination {
    page: Option<u64>,
    per_page: Option<u64>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, FromJsonQueryResult,
)]
pub struct DataVector(pub Vec<String>);
impl Default for DataVector {
    fn default() -> Self {
        DataVector(Vec::new())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct DataMap(pub BTreeMap<String, Vec<String>>);

impl Default for DataMap {
    fn default() -> Self {
        DataMap(BTreeMap::default())
    }
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromQueryResult, FromJsonQueryResult)]
pub struct MediaSelectResult2 {
    pub id: Option<i64>,
    pub storage_uri: Option<String>,
    pub sha256: Option<String>,
    pub perceptual_hash: Option<String>,
    pub uploaded: Option<DateTimeWithTimeZone>,
    pub created: Option<DateTimeWithTimeZone>,
    pub title: Option<String>,
    #[serde(default)]
    pub creators: DataVector,
    #[serde(default)]
    pub sources: DataVector,
    #[serde(default)]
    pub collections: DataVector,
    #[serde(default)]
    pub tag_groups: DataMap,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub result: Vec<MediaSelectResult2>,
}

async fn load_media_item(id: i64, db: &DatabaseConnection) -> Result<MediaSelectResult2, AppError> {
    let found_media = Media::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::Postgres,
            queries::MEDIA_QUERY_ID,
            vec![id.into()],
        ))
        .into_model::<MediaSelectResult2>()
        .one(db)
        .await?;
    Ok(found_media.expect("Media not found"))
}

pub async fn get_media(
    state: State<AppState>,
    pagination: Query<Pagination>,
) -> Result<(StatusCode, Json<SearchResult>), AppError> {
    let found_media = Media::find()
        .from_raw_sql(Statement::from_string(
            DbBackend::Postgres,
            queries::MEDIA_QUERY,
        ))
        .into_model::<MediaSelectResult2>()
        .paginate(&state.conn, pagination.per_page.unwrap_or_else(|| 50u64))
        .fetch_page(pagination.page.unwrap_or_else(|| 0))
        .await?;

    Ok((
        StatusCode::OK,
        Json(SearchResult {
            result: found_media,
        }),
    ))
}

async fn media_update(
    payload: MediaSelectResult2,
    new_model: &media::Model,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    // Handle Tag Groups and Tags
    let tag_tuple = tag_funcs::groups_to_tuple(&payload.tag_groups.0);
    if !tag_tuple.is_empty() {
        let new_groups = tag_funcs::tag_group_insert(&tag_tuple, db).await?;
        let inserted_tags = tag_funcs::tags_insert(&tag_tuple, &new_groups, db).await?;
        tag_funcs::tags_insert_relations(new_model.id, inserted_tags, db).await?;
    }
    tag_funcs::tags_update(tag_tuple, &new_model, db).await?;

    creators_insert(payload.creators.0.clone(), new_model.id, db).await?;
    creator_delete(payload.creators.0, new_model.id, db).await?;

    sources_insert(payload.sources.0.clone(), new_model.id, db).await?;
    sources_delete(payload.sources.0, new_model.id, db).await?;
    Ok(())
}

pub async fn post_media(
    state: State<AppState>,
    mut multipart: extract::Multipart,
) -> Result<(StatusCode, Json<Option<MediaSelectResult2>>), AppError> {
    let mut payload_option: Option<MediaSelectResult2> = None;
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
        let image_hash = HasherConfig::with_bytes_type::<[u8; 8]>()
            .hash_alg(Gradient)
            .hash_size(8, 8)
            .preproc_dct()
            .to_hasher()
            .hash_image(&im);
        let hash: [u8; 8] = image_hash.as_bytes().try_into()?;
        let phash = i64::from_be_bytes(hash);
        payload.perceptual_hash = Some(format!("{:x}", phash))
    }

    //Hash
    let mut hasher = Sha256::new();
    hasher.update(&file);
    let hash = hasher.finalize();
    println!("{:x}", hash);

    let mut file_name: std::path::PathBuf = std::path::PathBuf::new();
    file_name.set_file_name(format!("{:x}", hash));
    file_name.set_extension(&image_format);

    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_item: media::ActiveModel = media::ActiveModel {
        storage_uri: Set(file_name.to_string_lossy().to_string()),
        sha256: Set(format!("{:x}", hash)),
        perceptual_hash: Set(payload.perceptual_hash.clone()),
        created: Set(payload.created.clone()),
        title: Set(payload.title.clone()),
        r#type: Set(Some(image_format)),
        ..Default::default()
    };

    let new_model = new_item.insert(&txn).await?;

    media_update(payload, &new_model, &txn).await?;

    let mut out = File::create_new(state.storage_dir.clone().join(file_name))?;
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

pub async fn update_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<MediaSelectResult2>,
) -> Result<(StatusCode, Json<MediaSelectResult2>), AppError> {
    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_model = Media::update(media::ActiveModel {
        id: Set(id),
        perceptual_hash: Set(payload.perceptual_hash.clone()),
        created: Set(payload.created.clone()),
        title: Set(payload.title.clone()),
        ..Default::default()
    })
    .exec(&state.conn)
    .await?;

    media_update(payload, &new_model, &txn).await?;

    txn.commit().await?;
    //End of Transaction

    Ok((
        StatusCode::OK,
        Json(load_media_item(id, &state.conn).await?),
    ))
}

pub async fn get_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<MediaSelectResult2>), AppError> {
    Ok((
        StatusCode::OK,
        Json(load_media_item(id, &state.conn).await?),
    ))
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

pub async fn get_media_file(
    state: State<AppState>,
    Path(id): Path<i32>,
) -> Result<(StatusCode, Response), AppError> {
    println!("get_media called: id={}", id);
    let result = Media::find_by_id(id).one(&state.conn).await?;
    let media_item: media::Model = match result {
        Some(media_item) => media_item,
        None => {
            return Ok((
                StatusCode::NOT_FOUND,
                format!("Media ID {:?}", id).into_response(),
            ));
        }
    };

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

pub async fn get_media_thumbnail(
    state: State<AppState>,
    Path(id): Path<i32>,
) -> Result<(StatusCode, Response), AppError> {
    println!("get_media called: id={}", id);
    let result = Media::find_by_id(id).one(&state.conn).await?;
    let media_item: media::Model = match result {
        Some(media_item) => media_item,
        None => {
            return Ok((
                StatusCode::NOT_FOUND,
                format!("Media ID {:?}", id).into_response(),
            ));
        }
    };

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
