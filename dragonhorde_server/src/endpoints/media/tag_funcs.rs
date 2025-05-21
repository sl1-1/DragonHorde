use crate::error::AppError;
use entity::prelude::{MediaTags};
use entity::{media, media_tags};
use entity::{tag_groups, tag_groups::Entity as TagGroups};
use entity::{tags, tags::Entity as Tags};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, SelectColumns};
use sea_orm::{DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect, RelationTrait, Set};
use sea_query::JoinType;
use std::collections::{BTreeMap, HashMap, HashSet};

pub fn groups_to_tuple(tags_in: BTreeMap<String, Vec<String>>) -> Vec<(String, String)> {
    let mut out_tags: Vec<(String, String)> = tags_in
        .into_iter()
        .map(|tg| -> Vec<(String, String)> {
            tg.1.iter().map(|t| (tg.0.clone().to_lowercase(), t.to_lowercase())).collect()
        })
        .flatten()
        .collect();
    out_tags.sort_unstable_by_key(|tg| tg.1.clone());
    out_tags.dedup_by_key(|tg| tg.1.clone());
    out_tags
}

/// Insert any new tag groups, and return the respective models of both new and old tag groups
pub async fn tag_group_insert(
    mut groups_in: Vec<String>,
    db: &DatabaseTransaction,
) -> Result<HashMap<String, i64>, AppError> {
    groups_in.sort();
    groups_in.dedup();
    if !groups_in.is_empty() {
        let existing: Vec<(String, i64)> = TagGroups::find()
            .filter(tag_groups::Column::Name.is_in(&groups_in))
            .select_only()
            .select_column(tag_groups::Column::Name)
            .select_column(tag_groups::Column::Id)
            .into_tuple()
            .all(db)
            .await?;

        let (existing_groups, _): (Vec<_>, Vec<_>) = existing.clone().into_iter().unzip();


        let tg: Vec<tag_groups::ActiveModel> = groups_in
            .into_iter()
            .filter(|g| {!existing_groups.contains(&g)})
            .map(|i| tag_groups::ActiveModel {
                name: Set(i),
                ..Default::default()
            })
            .collect();

        let mut groups: HashMap<String, i64> = HashMap::from_iter(existing);

        if !tg.is_empty() {
            groups.extend(TagGroups::insert_many(tg)
                .exec_with_returning_many(db)
                .await?
                .into_iter()
                .map(|tg| (tg.name, tg.id)));
        }

        Ok(groups)
    }
    else {
        Ok(HashMap::new())
    }
}

/// Insert any new tags, and return a vector of the respective models of both new and old tag groups
pub async fn tags_insert(
    tags: &Vec<(String, String)>,
    db: &DatabaseTransaction,
) -> Result<Vec<i64>, AppError> {

    let tags_in = tags.clone();

    let mut tags_search: Vec<&String> = tags.iter().map(|t| &t.1).collect();
    tags_search.sort();
    tags_search.dedup();


    let existing: Vec<(String, i64)> = Tags::find()
        .filter(tags::Column::Tag.is_in(tags_search))
        .select_only()
        .select_column(tags::Column::Tag)
        .select_column(tags::Column::Id)
        .into_tuple()
        .all(db)
        .await?;


    dbg!(&existing);

    let (existing_tags, existing_ids): (Vec<_>, Vec<_>) = existing.into_iter().unzip();

    let groups = tag_group_insert(tags
                         .clone()
                         .into_iter()
                         .filter(|t| !existing_tags.contains(&t.1))
                         .map(|t| t.0)
                         .collect(),
                     db).await?;
    dbg!(&groups);


    let new_tags: Vec<tags::ActiveModel> = tags_in
        .into_iter()
        .filter(|t| {!existing_tags.contains(&t.1)})
        .map(|t| tags::ActiveModel {
            tag: Set(t.1),
            group: Set(groups.get(&t.0).unwrap().clone()),
            ..Default::default()
        })
        .collect();

    let mut tags_out = existing_ids;
    if !new_tags.is_empty() {
        tags_out.extend(Tags::insert_many(new_tags)
            .exec_with_returning_keys(db)
            .await?);
    }

    Ok(tags_out)
}

pub async fn media_tags_insert(
    media_id: i64,
    tags: Vec<i64>,
    db: &DatabaseTransaction,
) -> Result<Vec<media_tags::Model>, AppError> {
    let tag_relations: Vec<media_tags::ActiveModel> = tags
        .into_iter()
        .map(|t| media_tags::ActiveModel {
            media_id: Set(media_id),
            tag_id: Set(t),
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

pub async fn media_tags_delete(
    new_tags: Vec<(String, String)>,
    media_item: &media::Model,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    
    let new_tags: HashSet<String> =  new_tags.into_iter().map(|t| t.1).collect();
    
    let to_delete: Vec<i64> = MediaTags::find()
        .filter(media_tags::Column::MediaId.eq(media_item.id))
        .join(JoinType::LeftJoin, media_tags::Relation::Tags.def())
        .filter(tags::Column::Tag.is_not_in(new_tags))
        .select_only()
        .select_column(tags::Column::Id)
        .into_tuple()
        .all(db)
        .await?;
    
    MediaTags::delete_many()
        .filter(media_tags::Column::TagId.is_in(to_delete))
        .filter(media_tags::Column::MediaId.eq(media_item.id))
        .exec(db).await?;

    Ok(())
}