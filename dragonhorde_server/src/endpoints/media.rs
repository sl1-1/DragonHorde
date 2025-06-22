use sqlx::types::chrono::FixedOffset;
use std::collections::{BTreeMap, HashMap};

use crate::api_models::{ApiMedia, ApiMediaReturn, ImageMetadata, ImageResolution};
use crate::error::AppError;
use crate::error::AppError::{BadRequest, Exists, Internal, NotFound};
use crate::AppState;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use axum::Json;
use dragonhorde_common::hash::{perceptual, sha256};

use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use image::imageops::Lanczos3;
use image::ImageReader;
use sqlx::types::BitVec;
use sqlx::{Error, PgPool, Postgres, Transaction};
use std::fs::File;
use std::io::{Cursor, Write};
use tokio::io::AsyncReadExt;
use utoipa::ToSchema;
use crate::endpoints::shared::creators_create;

/// Check if the media item exists and return it as ApiMedia, raise a AppError:NotFound
async fn load_media_item(id: i64, db: &PgPool) -> Result<ApiMediaReturn, AppError> {
    let perceptual_hash: Option<BitVec> = None;
    let r = sqlx::query_file_as!(
        ApiMediaReturn,
        "sql/media_item_get.sqlx",
        &vec![id][..],
        perceptual_hash
    )
    .fetch_one(db)
    .await
    .map_err(|e| match e {
        Error::RowNotFound => NotFound(format!("media with id {} not found", id)),
        _ => Internal(e.into()),
    });
    Ok(r?)
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
) -> Result<Json<Option<ApiMediaReturn>>, AppError> {
    let mut payload: ApiMedia = serde_json::from_str(data.as_str())?;

    let reader = ImageReader::new(Cursor::new(&file.contents)).with_guessed_format()?;
    let image_format = reader
        .format()
        .ok_or(BadRequest("Can't Decode Image".to_string()))?;

    let im = reader.decode()?;
    if payload.perceptual_hash.is_none() {
        payload.perceptual_hash = Some(perceptual(&im))
    }

    let meta = ImageMetadata {
        resolution: ImageResolution {
            width: im.width(),
            height: im.height(),
        },
        bits_per_pixel: im.color().bits_per_pixel(),
        transparent: im.color().has_alpha(),
    };

    let hash = sha256(&file.contents);

    if let Some(_) = sqlx::query!("SELECT id FROM media WHERE sha256 = $1", &hash)
        .fetch_optional(&state.conn)
        .await?
    {
        return Err(Exists(format!(
            "media with sha256 {} already exists",
            &hash
        )));
    }

    let mut file_name: std::path::PathBuf = std::path::PathBuf::new();
    file_name.set_file_name(&hash);
    file_name.set_extension(&image_format.extensions_str()[0]);

    //Database Transaction
    let mut tx = state.conn.begin().await?;

    let id = sqlx::query_scalar!(r#"
        INSERT INTO media(storage_uri, sha256, created, title, description, type, perceptual_hash, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
"#,
    file_name.to_string_lossy().to_string(),
    &hash,
        payload.created as Option<chrono::DateTime<FixedOffset>>,
        payload.title,
        payload.description,
        image_format.extensions_str()[0].to_string(),
        BitVec::from_bytes(&payload.perceptual_hash.expect("perceptual_hash should be set").to_be_bytes()),
        serde_json::to_value(meta)?
    ).fetch_one(&mut *tx).await?;

    if let Some(tags) = payload.tag_groups {
        tags_insert(&tags.0, id, &mut tx).await?;
    }

    if let Some(creators) = payload.creators {
        creators_media_create(creators.0, id, &mut tx).await?;
    }

    if let Some(sources) = payload.sources {
        sources_insert(&sources.0, id, &mut tx).await?;
    }

    if let Some(collections) = payload.collections {
        collections_insert(&collections.0, id, &mut tx).await?;
    }

    let media_path = state.storage_dir.clone().join(&file_name);
    let mut media_file = File::create(&media_path)?;
    media_file.write_all(&file.contents)?;

    let mut thumbnail_path = state.thumbnail_dir.clone();
    thumbnail_path.push(hash);
    thumbnail_path.set_extension("webp");
    let thumbnail = im.resize(400, 400, Lanczos3);
    thumbnail.save(&thumbnail_path)?;

    //End of Transaction
    match tx.commit().await {
        Ok(_) => Ok(Json(Some(load_media_item(id, &state.conn).await?))),
        Err(e) => {
            //If the transaction fails, remove the files
            std::fs::remove_file(media_path).ok();
            std::fs::remove_file(thumbnail_path).ok();
            Err(AppError::from(e))
        }
    }
}

#[utoipa::path(patch, path = "/v1/media/{id}", request_body = ApiMedia , responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn media_item_patch(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiMedia>,
) -> Result<(), AppError> {
    let item = load_media_item(id, &state.conn).await?;
    let mut tx = state.conn.begin().await?;

    sqlx::query!(
        r#"UPDATE media SET created = $2, title = $3, description = $4 WHERE id = $1"#,
        id,
        payload.created.or(item.created) as Option<chrono::DateTime<FixedOffset>>,
        payload.title.or(item.title),
        payload.description.or(item.description.clone())
    )
    .execute(&mut *tx)
    .await?;

    if let Some(sources) = payload.sources {
        sources_insert(&sources.0, id, &mut tx).await?;
        sources_delete(&sources.0, id, &mut tx).await?;
    }

    if let Some(creators) = payload.creators {
        creators_media_create(creators.0.clone(), id, &mut tx).await?;
        creators_delete(creators.0, id, &mut tx).await?
    }

    if let Some(tags) = payload.tag_groups {
        tags_insert(&tags.0, id, &mut tx).await?;
        tags_delete(&tags.0, id, &mut tx).await?;
    }
    if let Some(collections) = payload.collections {
        collections_insert(&collections.0, id, &mut tx).await?;
        collections_delete(&collections.0, id, &mut tx).await?;
    }

    tx.commit().await?;
    //End of Transaction

    Ok(())
}

#[utoipa::path(get, path = "/v1/media/{id}", responses((status = OK, body = ApiMedia)), tags = ["media"]
)]
pub async fn get_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMediaReturn>, AppError> {
    Ok(Json(load_media_item(id, &state.conn).await?))
}

#[utoipa::path(delete, path = "/v1/media/{id}", responses((status = OK)), tags = ["media"]
)]
pub async fn delete_media_item(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(), AppError> {
    let item = load_media_item(id, &state.conn).await?;
    sqlx::query!(r#"DELETE FROM media WHERE id = $1"#, item.id)
        .execute(&state.conn)
        .await?;
    let path = &state.storage_dir.join(item.storage_uri.unwrap().clone());
    if path.exists() {
        std::fs::remove_file(path).ok();
    }
    let thumbnail_path = state.thumbnail_dir.join(format!("{}.webp", &item.sha256));
    if thumbnail_path.exists() {
        std::fs::remove_file(thumbnail_path).ok();
    }
    Ok(())
}

#[utoipa::path(get, path = "/v1/media/by_hash/{hash}", responses((status = OK, body = ApiMedia)), tags = ["media"])]
pub async fn get_media_item_by_hash(
    state: State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<ApiMediaReturn>, AppError> {
    let r = sqlx::query_scalar!("SELECT id FROM media WHERE sha256 = $1", &hash)
        .fetch_optional(&state.conn)
        .await?;
    if let Some(id) = r {
        Ok(Json(load_media_item(id, &state.conn).await?))
    } else {
        Err(NotFound(format!("media with hash {} not found", hash)))
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
    let r = sqlx::query!(
        r#"SELECT "storage_uri", "type" as "file_type" from media WHERE id = $1 "#,
        id
    )
    .fetch_one(&state.conn)
    .await?;

    let path = &state.storage_dir.join(r.storage_uri);
    let mut file = tokio::fs::File::open(path).await?;
    let mut data: Vec<u8> = Vec::new();
    file.read_to_end(&mut data).await?;

    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        extension_to_mime(&r.file_type.expect("media type missing")).parse()?,
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
    let media_item = sqlx::query!(
        r#"SELECT "storage_uri", "sha256", "type" as "file_type" from media WHERE id = $1 "#,
        id
    )
    .fetch_one(&state.conn)
    .await?;

    let mut data: Vec<u8> = Vec::new();

    let thumbnail_path = state
        .thumbnail_dir
        .join(format!("{}.webp", &media_item.sha256));
    if thumbnail_path.exists() {
        let mut file = tokio::fs::File::open(&thumbnail_path).await?;
        file.read_to_end(&mut data).await?;
    } else {
        let file_path = &state.storage_dir.join(media_item.storage_uri);
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

#[utoipa::path(get, path = "/v1/media/{id}/creators", responses((status = OK, body = Vec<String>)), tags = ["media"]
)]
pub async fn get_media_item_creators(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<String>>, AppError> {
    let r: Vec<String> = sqlx::query_file_scalar!("sql/media_item_get_creators.sqlx", id)
        .fetch_all(&state.conn)
        .await?;
    Ok(Json(r))
}

#[utoipa::path(get, path = "/v1/media/{id}/collections", responses((status = OK, body = Vec<String>)), tags = ["media"]
)]
pub async fn get_media_item_collections(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<String>>, AppError> {
    let r: Vec<String> = sqlx::query_file_scalar!("sql/media_item_get_collections.sqlx", id)
        .fetch_all(&state.conn)
        .await?;
    Ok(Json(r))
}

#[utoipa::path(get, path = "/v1/media/{id}/tags", responses((status = OK, body = HashMap<String, Vec<String>>)), tags = ["media"]
)]
pub async fn get_media_item_tags(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<HashMap<String, Vec<String>>>, AppError> {
    let r = sqlx::query_file!("sql/media_item_get_tags.sqlx", id)
        .fetch_all(&state.conn)
        .await?
        .into_iter()
        .map(|i| (i.name, i.ts.unwrap()))
        .collect::<HashMap<String, Vec<String>>>();
    Ok(Json(r))
}

pub async fn creators_media_create(
    creators_in: Vec<String>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<Vec<i64>, AppError> {
    let mut creators_in = creators_in.clone();
    creators_in.sort_by_key(|c| c.to_lowercase());
    creators_in.dedup_by_key(|c| c.to_lowercase());
    if !creators_in.is_empty() {
        let mut creators_inserted: Vec<i64> = creators_create(creators_in, db).await?;
        sqlx::query!(
            r#"INSERT INTO media_creators SELECT $1, * FROM unnest($2::bigint[]) ON CONFLICT DO NOTHING "#,
            id, &creators_inserted[..]
        ).execute(&mut **db).await?;

        Ok(creators_inserted)
    } else {
        Ok(vec![])
    }
}

async fn creators_delete(
    creators_in: Vec<String>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), AppError> {
    sqlx::query_file!(
        "sql/endpoints/media/creators_delete.sqlx",
        id,
        &creators_in
            .into_iter()
            .map(|s| s.to_lowercase())
            .collect::<Vec<String>>()
    )
    .execute(&mut **db)
    .await?;
    Ok(())
}

pub async fn collections_insert(
    collections_in: &Vec<String>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), AppError> {
    let mut collection_ids: Vec<i64> = Vec::new();
    for collection in collections_in {
        let r = sqlx::query_scalar!(
            r#"
            WITH RECURSIVE cte AS (SELECT id, name, parent, array [name] as path
                                   FROM collections
                                   WHERE parent is null
                                   UNION ALL
                                   SELECT c.id, c.name, c.parent, ct.path || c.name
                                   FROM cte ct
                                            JOIN
                                        collections c
                                        ON c.parent = ct.id)
            SELECT id
            FROM cte
            WHERE path = $1::varchar[]
            LIMIT 1
            "#,
            &collection
                .split("/")
                .map(|i| i.to_string())
                .collect::<Vec<String>>()[..]
        )
        .fetch_one(&mut **db)
        .await
        .map_err(|e| match e {
            Error::RowNotFound => BadRequest(format!("collection {} not found", collection)),
            _ => Internal(e.into()),
        })?;
        if let Some(r) = r {
            collection_ids.push(r);
        }
    }
    sqlx::query!(r#"INSERT into media_collection (media_id, collection_id) VALUES ($1, unnest($2::bigint[])) ON CONFLICT DO NOTHING"#, id, &collection_ids[..])
        .execute(&mut **db)
        .await?;
    Ok(())
}

pub async fn collections_delete(
    collections_in: &Vec<String>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), AppError> {
    if !collections_in.is_empty() {
        let mut collection_ids: Vec<i64> = Vec::new();
        for collection in collections_in {
            let r = sqlx::query_scalar!(
                r#"
            WITH RECURSIVE cte AS (SELECT id, name, parent, array [name] as path
                                   FROM collections
                                   WHERE parent is null
                                   UNION ALL
                                   SELECT c.id, c.name, c.parent, ct.path || c.name
                                   FROM cte ct
                                            JOIN
                                        collections c
                                        ON c.parent = ct.id)
            SELECT id
            FROM cte
            WHERE path = $1::varchar[]
            LIMIT 1
            "#,
                &collection
                    .split("/")
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()[..]
            )
            .fetch_one(&mut **db)
            .await?;
            dbg!(&r);
            if let Some(result) = r {
                collection_ids.push(result);
            }
        }
        sqlx::query!(r#"DELETE FROM media_collection WHERE media_id = $1 AND collection_id != any($2::bigint[]) "#, id, &collection_ids[..])
            .execute(&mut **db)
            .await?;
    } else {
        sqlx::query!(r#"DELETE FROM media_collection WHERE media_id = $1"#, id)
            .execute(&mut **db)
            .await?;
    }

    Ok(())
}

async fn sources_insert(
    sources: &Vec<String>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
                    INSERT INTO sources (media_id, source)
                    VALUES ($1, UNNEST($2::text[]))
                    ON CONFLICT DO NOTHING"#,
        id,
        &sources[..]
    )
    .execute(&mut **db)
    .await?;
    Ok(())
}

async fn sources_delete(
    sources: &Vec<String>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
                   DELETE
                   FROM sources
                   WHERE media_id = $1
                     AND source != ANY ($2::varchar[])"#,
        id,
        &sources[..]
    )
    .execute(&mut **db)
    .await?;

    Ok(())
}

async fn tags_insert(
    tags: &BTreeMap<String, Vec<String>>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), AppError> {
    let mut tag_tuple: Vec<(String, String)> = tags
        .into_iter()
        .map(|tg| -> Vec<(String, String)> {
            tg.1.iter()
                .map(|t| (tg.0.clone().to_lowercase(), t.to_lowercase()))
                .collect()
        })
        .flatten()
        .collect();
    tag_tuple.sort_unstable_by_key(|tg| tg.1.clone());
    tag_tuple.dedup_by_key(|tg| tg.1.clone());

    let tags_in = tag_tuple.clone();

    let mut tags_search: Vec<String> = tags_in.iter().map(|t| t.1.clone()).collect();
    tags_search.sort();
    tags_search.dedup();

    let existing: Vec<(String, i64)> = sqlx::query!(
        r#"SELECT tag, id FROM tags WHERE tag = any($1::varchar[])"#,
        &tags_search[..]
    )
    .fetch_all(&mut **db)
    .await?
    .into_iter()
    .map(|i| (i.tag, i.id))
    .collect();

    dbg!(&existing);

    let (existing_tags, mut existing_ids): (Vec<_>, Vec<_>) = existing.into_iter().unzip();

    let mut groups_in: Vec<String> = tag_tuple.clone().into_iter().map(|t| t.0.clone()).collect();
    groups_in.sort();
    groups_in.dedup();

    let mut groups: HashMap<String, i64> = HashMap::new();

    if !groups_in.is_empty() {
        let existing: Vec<(String, i64)> = sqlx::query!(
            r#"SELECT name, id FROM tag_groups WHERE name = any($1::varchar[])"#,
            &groups_in[..]
        )
        .fetch_all(&mut **db)
        .await?
        .into_iter()
        .map(|i| (i.name, i.id))
        .collect();
        let (existing_groups, _): (Vec<_>, Vec<_>) = existing.clone().into_iter().unzip();
        let new_groups: Vec<String> = groups_in
            .into_iter()
            .filter(|g| !existing_groups.contains(&g))
            .collect();
        groups.extend(existing);
        if !new_groups.is_empty() {
            let results: HashMap<String, i64> = sqlx::query!(
                r#"
                INSERT INTO tag_groups(name) VALUES (unnest($1::varchar[])) RETURNING name, id
            "#,
                &new_groups[..]
            )
            .fetch_all(&mut **db)
            .await?
            .into_iter()
            .map(|i| (i.name, i.id))
            .collect();
            groups.extend(results);
        }
        let (new_tags, new_groups): (Vec<_>, Vec<_>) = tag_tuple
            .into_iter()
            .filter(|t| !existing_tags.contains(&t.1))
            .map(|t| (t.1, groups.get(&t.0).unwrap().clone()))
            .collect();
        dbg!(&new_tags, &new_groups);
        existing_ids.extend(
            sqlx::query_scalar!(
                r#"
                INSERT INTO tags(tag, "group") SELECT * FROM unnest($1::varchar[], $2::bigint[])
                RETURNING id
            "#,
                &new_tags[..],
                &new_groups[..]
            )
            .fetch_all(&mut **db)
            .await?,
        );
    }
    sqlx::query!(
        "INSERT INTO media_tags(media_id, tag_id) SELECT $1, * FROM unnest($2::bigint[]) ON CONFLICT DO NOTHING ",
        id,
        &existing_ids[..]
    )
    .execute(&mut **db)
    .await?;
    Ok(())
}

async fn tags_delete(
    tags: &BTreeMap<String, Vec<String>>,
    id: i64,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), AppError> {
    let mut new_tags = tags
        .values()
        .flatten()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    new_tags.sort();
    new_tags.dedup();
    sqlx::query_file!("sql/endpoints/media/tags_delete.sqlx", id, &new_tags[..])
        .execute(&mut **db)
        .await?;
    Ok(())
}
