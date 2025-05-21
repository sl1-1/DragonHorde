use crate::error::AppError;
use entity::prelude::MediaTags;
use entity::{media, media_tags};
use entity::{tag_groups, tag_groups::Entity as TagGroups};
use entity::{tags, tags::Entity as Tags};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, SelectColumns};
use sea_orm::{DatabaseTransaction, EntityTrait, ModelTrait, QueryFilter, QuerySelect, RelationTrait, Set};
use sea_query::JoinType;
use std::collections::{BTreeMap, HashMap, HashSet};

pub fn groups_to_tuple(tags_in: &BTreeMap<String, Vec<String>>) -> Vec<(String, String)> {
    let mut ltags: Vec<(String, String)> = tags_in
        .iter()
        .map(|tg| -> Vec<(String, String)> {
            tg.1.iter().map(|t| (tg.0.clone().to_lowercase(), t.clone().to_lowercase())).collect()
        })
        .flatten()
        .collect();
    ltags.sort_unstable_by_key(| tg| tg.1.clone().to_lowercase());
    ltags.dedup_by_key(| tg| tg.1.clone().to_lowercase());
    ltags
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
    deduped_groups.sort_by_key(|g| g.to_lowercase());
    deduped_groups.dedup_by_key(|g| g.to_lowercase());
    if !deduped_groups.is_empty() {
        dbg!(&deduped_groups);
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
    else {
        Ok(HashMap::new())
    }
}

/// Insert any new tags, and return a vector of the respective models of both new and old tag groups
pub async fn tags_insert(
    tags: &Vec<(String, String)>,
    groups: &HashMap<String, i64>,
    db: &DatabaseTransaction,
) -> Result<Vec<tags::Model>, AppError> {

    let mut ltags = tags.clone();
    ltags.sort_unstable_by_key(| tg| tg.1.clone().to_lowercase());
    ltags.dedup_by_key(| tg| tg.1.clone().to_lowercase());

    let new_tags: Vec<tags::ActiveModel> = ltags
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
            OnConflict::column(tags::Column::Tag)
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
    let current_tags: Vec<(i64, String, String)> = MediaTags::find()
        .filter(media_tags::Column::MediaId.eq(media_item.id))
        .join(JoinType::LeftJoin, media_tags::Relation::Tags.def())
        .join(JoinType::LeftJoin, tag_groups::Relation::Tags.def().rev())
        .select_only()
        .select_column(tags::Column::Id)
        .select_column(tag_groups::Column::Name)
        .select_column(tags::Column::Tag)
        .into_tuple()
        .all(db)
        .await?;

    dbg!(&current_tags);

    let current_tags_id_by_tag: HashMap<String, i64> = current_tags
        .iter()
        .map(|t| (t.2.clone(), t.0))
        .collect();

    let current_tag_tup = current_tags
        .iter()
        .map(|t| -> (String, String) { (t.1.clone(), t.2.clone()) })
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