mod creator_funcs;
mod source_funcs;
mod tag_funcs;

pub use tag_funcs::{media_add_tag, media_delete_tag, media_get_tags};

use crate::AppState;
use crate::endpoints::media::creator_funcs::{creator_delete, creators_insert};
use crate::endpoints::media::source_funcs::{sources_delete, sources_insert};
use crate::error::AppError;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, extract};
use entity::media_collection;
use entity::media_creators;
use entity::media_tags;
use entity::sources;
use entity::tag_groups;
use entity::tags;
use entity::{collections, creators};
use entity::{media, media::Entity as Media};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, FromQueryResult, IntoActiveModel, JoinType, QuerySelect, RelationTrait, SelectColumns, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{Cursor, Write};
use img_hash::HashAlg::Gradient;
use img_hash::HasherConfig;
use tokio::io::AsyncReadExt;

#[derive(Debug, Deserialize, FromQueryResult)]
pub struct MediaSelectResult {
    pub id: i64,
    pub storage_uri: Option<String>,
    pub sha256: Option<String>,
    pub perceptual_hash: Option<String>,
    pub uploaded: Option<DateTimeWithTimeZone>,
    pub created: Option<DateTimeWithTimeZone>,
    pub title: Option<String>,
    pub creator: Option<String>,
    pub group: Option<String>,
    pub tag: Option<String>,
    pub source: Option<String>,
    pub collection: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaSchema {
    // #[serde(skip_deserializing)]
    pub id: Option<i64>,
    #[serde(skip_deserializing)]
    pub storage_uri: Option<String>,
    #[serde(skip_deserializing)]
    pub sha256: Option<String>,
    pub perceptual_hash: Option<String>,
    #[serde(skip_deserializing)]
    pub uploaded: Option<DateTimeWithTimeZone>,
    pub created: Option<DateTimeWithTimeZone>,
    pub title: Option<String>,
    pub tag_groups: Option<HashMap<String, Vec<String>>>,
    pub creators: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
    pub collections: Option<Vec<String>>,
}

impl From<media::Model> for MediaSchema {
    fn from(value: media::Model) -> Self {
        MediaSchema {
            id: Some(value.id),
            storage_uri: Some(value.storage_uri),
            sha256: Some(value.sha256),
            perceptual_hash: value.perceptual_hash,
            uploaded: Some(value.uploaded),
            created: value.created,
            title: value.title,
            tag_groups: None,
            creators: None,
            sources: None,
            collections: None,
        }
    }
}

mod update_model {
    use crate::endpoints::media::MediaSchema;
    use crate::endpoints::media::media::ActiveModel;
    use sea_orm::DeriveIntoActiveModel;
    use sea_orm::prelude::DateTimeWithTimeZone;

    #[derive(Debug, DeriveIntoActiveModel)]
    pub struct MediaschemaToActive {
        // pub id: i64,
        pub perceptual_hash: Option<String>,
        pub created: Option<DateTimeWithTimeZone>,
        pub title: Option<String>,
    }
    impl From<MediaSchema> for MediaschemaToActive {
        fn from(value: MediaSchema) -> Self {
            MediaschemaToActive {
                perceptual_hash: value.perceptual_hash,
                created: value.created,
                title: value.title,
            }
        }
    }
}

impl From<Vec<MediaSelectResult>> for MediaSchema {
    fn from(value: Vec<MediaSelectResult>) -> Self {
        let mut creators: Vec<String> = Vec::new();
        let mut sources: Vec<String> = Vec::new();
        let mut collections: Vec<String> = Vec::new();
        let mut taggroups: HashMap<String, Vec<String>> = HashMap::new();
        for i in &value {
            match &i.creator {
                Some(creator) => {
                    if !creators.contains(&creator) {
                        creators.push(creator.clone());
                    }
                }
                None => {}
            }
            match &i.group {
                Some(group) => match taggroups.get_mut(group) {
                    Some(tags) => {
                        tags.push(i.tag.clone().unwrap());
                    }
                    None => {
                        taggroups.insert(group.clone(), vec![i.tag.clone().unwrap()]);
                    }
                },
                None => {}
            }
            match &i.source {
                Some(source) => {
                    if !sources.contains(&source) {
                        sources.push(source.clone());
                    }
                }
                None => {}
            }
            match &i.collection {
                Some(collection) => {
                    if !collections.contains(&collection) {
                        collections.push(collection.clone());
                    }
                }
                None => {}
            }
        }
        MediaSchema {
            id: Some(value[0].id),
            storage_uri: value[0].storage_uri.clone(),
            sha256: value[0].sha256.clone(),
            perceptual_hash: value[0].perceptual_hash.clone(),
            uploaded: value[0].uploaded,
            created: value[0].created,
            title: value[0].title.clone(),
            tag_groups: Some(taggroups),
            creators: Some(creators),
            sources: Some(sources),
            collections: Some(collections),
        }
    }
}

async fn load_media_item(id: i64, db: &DatabaseConnection) -> Result<MediaSchema, AppError> {
    let found_media = Media::find_by_id(id)
        .join(JoinType::LeftJoin, media::Relation::Sources.def())
        .join(JoinType::LeftJoin, media::Relation::MediaTags.def())
        .join(JoinType::LeftJoin, media::Relation::MediaCollection.def())
        .join(JoinType::LeftJoin, media_tags::Relation::Tags.def())
        .join(JoinType::LeftJoin, tags::Relation::TagGroups.def())
        .join(JoinType::LeftJoin, media::Relation::MediaCreators.def())
        .join(JoinType::LeftJoin, media_creators::Relation::Creators.def())
        .join(
            JoinType::LeftJoin,
            media_collection::Relation::Collections.def(),
        )
        .select_column_as(tag_groups::Column::Name, "group")
        .select_column_as(collections::Column::Name, "collection")
        .select_column(tags::Column::Tag)
        .select_column(sources::Column::Source)
        .select_column_as(creators::Column::Name, "creator")
        .into_model::<MediaSelectResult>()
        .all(db)
        .await?;
    Ok(MediaSchema::from(found_media))
}

pub async fn get_media(
    state: State<AppState>,
) -> Result<(StatusCode, Json<Vec<MediaSchema>>), AppError> {
    let found_media = Media::find()
        .join(JoinType::LeftJoin, media::Relation::Sources.def())
        .join(JoinType::LeftJoin, media::Relation::MediaTags.def())
        .join(JoinType::LeftJoin, media::Relation::MediaCollection.def())
        .join(JoinType::LeftJoin, media_tags::Relation::Tags.def())
        .join(JoinType::LeftJoin, tags::Relation::TagGroups.def())
        .join(JoinType::LeftJoin, media::Relation::MediaCreators.def())
        .join(JoinType::LeftJoin, media_creators::Relation::Creators.def())
        .join(
            JoinType::LeftJoin,
            media_collection::Relation::Collections.def(),
        )
        .select_column_as(tag_groups::Column::Name, "group")
        .select_column_as(collections::Column::Name, "collection")
        .select_column(tags::Column::Tag)
        .select_column(sources::Column::Source)
        .select_column_as(creators::Column::Name, "creator")
        .into_model::<MediaSelectResult>()
        .all(&state.conn)
        .await?;

    let mut found_media_by_id: BTreeMap<i64, Vec<MediaSelectResult>> = BTreeMap::new();
    for item in found_media {
        match found_media_by_id.get_mut(&item.id) {
            Some(i) => {
                i.push(item);
            }
            None => {
                found_media_by_id.insert(item.id, vec![item]);
            }
        }
    }

    Ok((
        StatusCode::OK,
        Json(
            found_media_by_id
                .into_iter()
                .map(|m| MediaSchema::from(m.1))
                .collect(),
        ),
    ))
}

async fn media_update(
    payload: MediaSchema,
    new_model: &media::Model,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    // Handle Tag Groups and Tags
    if let Some(new_tag_groups) = payload.tag_groups {
        let tag_tuple = tag_funcs::groups_to_tuple(&new_tag_groups);

        let new_groups = tag_funcs::tag_group_insert(&tag_tuple, db).await?;

        let inserted_tags = tag_funcs::tags_insert(&tag_tuple, &new_groups, db).await?;

        tag_funcs::tags_insert_relations(new_model.id, inserted_tags, db).await?;

        tag_funcs::tags_update(tag_tuple, &new_model, db).await?;
    }

    if let Some(creators_in) = &payload.creators {
        dbg!(&creators_in);
        creators_insert(creators_in.clone(), new_model.id, db).await?;
    }
    creator_delete(payload.creators, new_model.id, db).await?;

    if let Some(sources_in) = &payload.sources {
        sources_insert(sources_in.clone(), new_model.id, db).await?;
    }
    sources_delete(payload.sources, new_model.id, db).await?;
    Ok(())
}

pub async fn post_media(
    state: State<AppState>,
    mut multipart: extract::Multipart,
) -> Result<(StatusCode, Json<Option<MediaSchema>>), AppError> {
    let mut payload_option: Option<MediaSchema> = None;
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

    let payload = match payload_option {
        Some(payload) => payload,
        None => return Ok((StatusCode::BAD_REQUEST, Json(None))),
    };

    let file = match file_option {
        Some(file) => file,
        None => return Ok((StatusCode::BAD_REQUEST, Json(None))),
    };


    let reader = image::io::Reader::new(Cursor::new(&file)).with_guessed_format()?;
    let im = reader.decode()?;

    let image_hash =
        HasherConfig::with_bytes_type::<[u8; 8]>()
            .hash_alg(Gradient)
            .hash_size(8, 8)
            .preproc_dct()
            .to_hasher()
            .hash_image(&im);
    let hash: [u8; 8] = image_hash.as_bytes().try_into()?;
    let phash = i64::from_be_bytes(hash);

    //Hash
    let mut hasher = Sha256::new();
    hasher.update(&file);
    let hash = hasher.finalize();
    println!("{:x}", hash);

    let kind = infer::get(&file).expect("file type is known");

    let mut file_name: std::path::PathBuf = std::path::PathBuf::new();
    file_name.set_file_name(format!("{:x}", hash));
    file_name.set_extension(kind.extension());

    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_item: media::ActiveModel = media::ActiveModel {
        storage_uri: Set(file_name.to_string_lossy().to_string()),
        sha256: Set(format!("{:x}", hash)),
        perceptual_hash: Set(Some(format!("{:x}", phash))),
        created: Set(payload.created.clone()),
        title: Set(payload.title.clone()),
        ..Default::default()
    };

    let new_model = new_item.insert(&txn).await?;

    media_update(payload, &new_model, &txn).await?;

    let mut out = File::create_new(state.storage_dir.clone().join(file_name))?;
    out.write_all(&file)?;

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
    Json(payload): Json<MediaSchema>,
) -> Result<(StatusCode, Json<MediaSchema>), AppError> {
    let mut active_model =
        update_model::MediaschemaToActive::from(payload.clone()).into_active_model();
    active_model.id = Set(id);

    //Database Transaction
    let txn: DatabaseTransaction = state.conn.begin().await?;

    let new_model = Media::update(active_model.into_active_model())
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
) -> Result<(StatusCode, Json<MediaSchema>), AppError> {
    Ok((
        StatusCode::OK,
        Json(load_media_item(id, &state.conn).await?),
    ))
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
    let kind = infer::get(&data).expect("file type is known");
    let body = Body::from(data);

    let headers = [
        (header::CONTENT_TYPE, kind.mime_type()),
        (
            header::CONTENT_DISPOSITION,
            &format!("attachment; filename=\"{:?}\"", media_item.storage_uri),
        ),
    ];
    Ok((StatusCode::OK, (headers, body).into_response()))
}
