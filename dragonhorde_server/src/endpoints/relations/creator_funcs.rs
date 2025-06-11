use crate::error::AppError;
use entity::{creator_alias, creator_alias::Entity as CreatorAlias};
use entity::{creators, creators::Entity as Creators};

use entity::{media_creators, media_creators::Entity as MediaCreators};
use entity::{collection_creators, collection_creators::Entity as CollectionCreators};

use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ColumnTrait, DatabaseTransaction, EntityTrait, JoinType, QuerySelect, RelationTrait, Set,
};
use sea_orm::{QueryFilter, SelectColumns};

async fn creators_new(
    creators_in: Vec<String>,
    db: &DatabaseTransaction,
) -> Result<Vec<i64>, AppError> {
    let mut creators_in = creators_in.clone();
    creators_in.sort_by_key(|c| c.to_lowercase());
    creators_in.dedup_by_key(|c| c.to_lowercase());
    if !creators_in.is_empty() {
        dbg!(&creators_in);
        // Look through existing Creator Aliases, returning id and aliases that exist
        let existing: Vec<(i64, String)> = CreatorAlias::find()
            .filter(
                creator_alias::Column::Alias.is_in(
                    &creators_in
                        .iter()
                        .map(|s| s.clone().to_lowercase())
                        .collect::<Vec<String>>(),
                ),
            )
            .select_only()
            .select_column(creator_alias::Column::Creator)
            .select_column(creator_alias::Column::Alias)
            .into_tuple()
            .all(db)
            .await?;
        println!("{}", &creators_in[0]);

        let (existing_id, existing_name): (Vec<_>, Vec<_>) = existing.into_iter().unzip();

        //Create the models to be inserted that don't exist.
        // Filtering using the results from the first step
        let creators_to_insert: Vec<creators::ActiveModel> = creators_in
            .clone()
            .into_iter()
            .filter(|c| !existing_name.contains(&c.to_lowercase()))
            .map(|s| creators::ActiveModel {
                name: Set(s),
                ..Default::default()
            })
            .collect();

        dbg!(&creators_to_insert);
        let mut creators_inserted = Vec::new();
        if creators_to_insert.len() > 0 {
            //Insert the creators that don't already exist
            creators_inserted = Creators::insert_many(creators_to_insert)
                .on_conflict(
                    OnConflict::column(creators::Column::Name)
                        .update_column(creators::Column::Name)
                        .to_owned(),
                )
                .exec_with_returning_keys(db)
                .await?;
        }

        //Merge the newly created creators, and the existing creator ids
        creators_inserted.extend(existing_id);

        creators_inserted.sort();
        creators_inserted.dedup();
        Ok(creators_inserted)
    }
    else {
        Ok(vec![])
    }
}

/// Check if the creators provided exist in the creator_alias table
/// creator_alias.alias is stored as all lower case to make comparisons easier
/// When creating the creators table entry, maintain case, as it isn't used for searching
pub async fn media_creators_create(
    creators_in: Vec<String>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    if !creators_in.is_empty() {
        let creators_inserted = creators_new(creators_in, db).await?;
        
        let existing_relations: Vec<i64> = MediaCreators::find()
            .filter(media_creators::Column::MediaId.eq(id))
            .filter(media_creators::Column::CreatorId.is_in(creators_inserted.clone()))
            .select_only()
            .column(media_creators::Column::CreatorId)
            .into_tuple()
            .all(db).await?;
        
        //Create the new relations
        let creators_relations: Vec<media_creators::ActiveModel> = creators_inserted
            .into_iter()
            .filter(|c| !existing_relations.contains(c))
            .map(|c| media_creators::ActiveModel {
                media_id: Set(id),
                creator_id: Set(c),
            })
            .collect();

        dbg!(&creators_relations);
        //Insert the relations
        if creators_relations.len() > 0 {
            MediaCreators::insert_many(creators_relations)
                .exec_with_returning_many(db).await?;
        }
        Ok(())
    } else {
        Ok(())
    }
}

/// Check if the creators provided exist in the creator_alias table
/// creator_alias.alias is stored as all lower case to make comparisons easier
/// When creating the creators table entry, maintain case, as it isn't used for searching
pub async fn collection_creators_create(
    creators_in: Vec<String>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    if !creators_in.is_empty() {
        let creators_inserted = creators_new(creators_in, db).await?;

        let existing_relations: Vec<i64> = CollectionCreators::find()
            .filter(collection_creators::Column::CollectionId.eq(id))
            .filter(collection_creators::Column::CreatorId.is_in(creators_inserted.clone()))
            .select_only()
            .column(collection_creators::Column::CreatorId)
            .into_tuple()
            .all(db).await?;

        //Create the new relations
        let creators_relations: Vec<collection_creators::ActiveModel> = creators_inserted
            .into_iter()
            .filter(|c| !existing_relations.contains(c))
            .map(|c| collection_creators::ActiveModel {
                collection_id: Set(id),
                creator_id: Set(c),
            })
            .collect();

        dbg!(&creators_relations);
        //Insert the relations
        if creators_relations.len() > 0 {
            CollectionCreators::insert_many(creators_relations)
                .exec_with_returning_many(db).await?;
        }
        Ok(())
    } else {
        Ok(())
    }
}

pub async fn media_creators_delete(
    creators_in: Vec<String>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    let out: Vec<i64> = MediaCreators::find()
        .join(JoinType::LeftJoin, media_creators::Relation::Creators.def())
        .join(JoinType::LeftJoin, creators::Relation::CreatorAlias.def())
        .filter(
            creator_alias::Column::Alias.is_not_in(
                creators_in
                    .into_iter()
                    .map(|s| s.to_lowercase())
                    .collect::<Vec<String>>(),
            ),
        )
        .filter(media_creators::Column::MediaId.eq(id))
        .select_only()
        .select_column(creator_alias::Column::Creator)
        .into_tuple()
        .all(db)
        .await?;
    MediaCreators::delete_many()
        .filter(media_creators::Column::CreatorId.is_in(out))
        .filter(media_creators::Column::MediaId.eq(id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn collection_creators_delete(
    creators_in: Vec<String>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    let out: Vec<i64> = CollectionCreators::find()
        .join(JoinType::LeftJoin, collection_creators::Relation::Creators.def())
        .join(JoinType::LeftJoin, creators::Relation::CreatorAlias.def())
        .filter(
            creator_alias::Column::Alias.is_not_in(
                creators_in
                    .into_iter()
                    .map(|s| s.to_lowercase())
                    .collect::<Vec<String>>(),
            ),
        )
        .filter(collection_creators::Column::CollectionId.eq(id))
        .select_only()
        .select_column(creator_alias::Column::Creator)
        .into_tuple()
        .all(db)
        .await?;
    CollectionCreators::delete_many()
        .filter(collection_creators::Column::CreatorId.is_in(out))
        .filter(collection_creators::Column::CollectionId.eq(id))
        .exec(db)
        .await?;
    Ok(())
}
