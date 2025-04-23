use sea_orm::{FromQueryResult, QueryFilter, RelationTrait};
use std::collections::{HashMap, HashSet};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, JoinType, ModelTrait, QuerySelect, SelectColumns, Set, TransactionTrait};
use sea_orm::sea_query::OnConflict;
use serde::Deserialize;
use entity::prelude::{MediaTags};
use entity::{media, media_tags};
use entity::{tags, tags::Entity as Tags};
use entity::{tag_groups, tag_groups::Entity as TagGroups};
use crate::AppState;
use crate::error::AppError;

#[derive(Debug, Deserialize, FromQueryResult)]
pub struct TagEntity {
    group: String,
    tag: String,
}


pub fn groups_to_tuple(tags_in: &HashMap<String, Vec<String>>) -> Vec<(String, String)> {
    tags_in
        .iter()
        .map(|tg| -> Vec<(String, String)> {
            tg.1.iter().map(|t| (tg.0.clone(), t.clone())).collect()
        })
        .flatten()
        .collect()
}

/// Insert any new tag groups, and return the respective models of both new and old tag groups
pub async fn tag_group_insert(
    tags: &Vec<(String, String)>,
    db: &DatabaseTransaction,
) -> Result<HashMap<String, i64>, AppError> {
    let mut deduped_groups: Vec<String> = tags
        .clone()
        .into_iter()
        .map(|i| i.0)
        .collect();
    deduped_groups.dedup();
    let tg: Vec<tag_groups::ActiveModel>;
    tg = deduped_groups
        .clone()
        .into_iter()
        .map(|i| tag_groups::ActiveModel {
            name: Set(i),
            ..Default::default()
        })
        .collect();

    //Not sure if doing an insert here is correct, or if existence should be checked first
    let ret = TagGroups::insert_many(tg)
        .on_conflict(
            OnConflict::column(tag_groups::Column::Name)
                .update_column(tag_groups::Column::Name)
                .to_owned(),
        )
        .exec_with_returning_many(db)
        .await?;

    Ok(ret.iter().map(|tg| (tg.name.clone(), tg.id)).collect())
}

/// Insert any new tags, and return a vector of the respective models of both new and old tag groups
pub async fn tags_insert(
    tags: &Vec<(String, String)>,
    groups: &HashMap<String, i64>,
    db: &DatabaseTransaction,
) -> Result<Vec<tags::Model>, AppError> {
    let new_tags: Vec<tags::ActiveModel> = tags
        .iter()
        .map(|t| tags::ActiveModel {
            tag: Set(t.1.clone()),
            group: Set(groups.get(&t.0).unwrap().clone()),
            ..Default::default()
        })
        .collect();

    //Not sure if doing an insert here is correct, or if existence should be checked first
    Ok(Tags::insert_many(new_tags)
        .on_conflict(
            OnConflict::columns([tags::Column::Tag, tags::Column::Group])
                .update_column(tags::Column::Tag)
                .to_owned(),
        )
        .exec_with_returning_many(db)
        .await?)
}

pub async fn tags_insert_relations(
    media_id: i64,
    tags: Vec<tags::Model>,
    db: &DatabaseTransaction,
) -> Result<Vec<media_tags::Model>, AppError> {
    let tag_relations: Vec<media_tags::ActiveModel> = tags
        .into_iter()
        .map(|t: tags::Model| media_tags::ActiveModel {
            media_id: Set(media_id),
            tag_id: Set(t.id),
        })
        .collect();
    Ok(MediaTags::insert_many(tag_relations)
        .on_conflict(
            OnConflict::columns([media_tags::Column::MediaId, media_tags::Column::TagId])
                .do_nothing()
                .to_owned(),
        )
        .exec_with_returning_many(db)
        .await?)
}

pub async fn tags_update(
    new_tags: Vec<(String, String)>,
    media_item: &media::Model,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    let current_tags = media_item
        .find_related(Tags)
        .find_also_related(TagGroups)
        .all(db)
        .await?;

    dbg!(&current_tags);

    let current_tags_id_by_tag: HashMap<String, i64> = current_tags
        .iter()
        .map(|t| (t.0.tag.clone(), t.0.id))
        .collect();

    let current_tag_tup = current_tags
        .iter()
        .map(|t| -> (String, String) { (t.1.clone().unwrap().name, t.0.tag.clone()) })
        .collect::<Vec<(String, String)>>();

    let current_hash: HashSet<(String, String)> = current_tag_tup.into_iter().collect();
    let new_hash: HashSet<(String, String)> = new_tags.into_iter().collect();

    let to_delete = current_hash.symmetric_difference(&new_hash);

    dbg!(&current_hash);
    dbg!(&new_hash);
    dbg!(&to_delete);

    for tag in to_delete {
        MediaTags::delete_by_id((
            media_item.id,
            current_tags_id_by_tag.get(&tag.1).unwrap().clone(),
        ))
            .exec(db)
            .await?;
    }

    Ok(())
}

pub async fn media_get_tags(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<HashMap<String, Vec<String>>>), AppError> {
    let found_tag = Tags::find()
        .select_only()
        .join(JoinType::LeftJoin, tags::Relation::MediaTags.def())
        .join(JoinType::LeftJoin, tags::Relation::TagGroups.def())
        .filter(media_tags::Column::MediaId.eq(id))
        .select_column_as(tag_groups::Column::Name, "group")
        .select_column(tags::Column::Tag)
        .into_model::<TagEntity>()
        .all(&state.conn)
        .await?;
    dbg!(&found_tag);

    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    for tag in found_tag {
        match tags.get_mut(&tag.group) {
            Some(tags) => {
                tags.push(tag.tag);
            }
            None => {
                tags.insert(tag.group, vec![tag.tag]);
            }
        }
    }

    Ok((StatusCode::OK, Json(tags)))
}

pub async fn media_add_tag(
    state: State<AppState>,
    Path(id): Path<i64>,
    tag: Query<TagEntity>,
) -> Result<StatusCode, AppError> {
    //Database Transaction
    let tag_tup = &vec![(tag.group.clone(), tag.tag.clone())];
    let txn: DatabaseTransaction = state.conn.begin().await?;
    let group = tag_group_insert(tag_tup, &txn).await?;
    let tags = tags_insert(tag_tup, &group, &txn).await?;
    let inserted = tags_insert_relations(id, tags, &txn).await?;
    txn.commit().await?;
    //End of Transaction
    dbg!(&inserted);
    if inserted.len() == 0 {
        return Ok(StatusCode::CONFLICT);
    }
    Ok(StatusCode::CREATED)
}
pub async fn media_delete_tag(
    state: State<AppState>,
    Path(id): Path<i64>,
    tag: Query<TagEntity>,
) -> Result<StatusCode, AppError> {
    let found_tag = Tags::find()
        .find_with_related(TagGroups)
        .filter(tags::Column::Tag.like(&tag.tag))
        .filter(tag_groups::Column::Name.like(&tag.group))
        .all(&state.conn)
        .await?;
    MediaTags::delete_by_id((id, found_tag[0].0.id))
        .exec(&state.conn)
        .await?;

    Ok(StatusCode::OK)
}