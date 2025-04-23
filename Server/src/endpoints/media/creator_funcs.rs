use crate::error::AppError;
use entity::{creators, creators::Entity as Creators};
use entity::{media_creators, media_creators::Entity as MediaCreators};
use sea_orm::QueryFilter;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ColumnTrait, DatabaseTransaction, EntityTrait, JoinType, QuerySelect, RelationTrait, Set,
};

pub async fn creators_insert(
    creators_in: Vec<String>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<Vec<media_creators::Model>, AppError> {
    let creators_to_insert: Vec<creators::ActiveModel> = creators_in
        .into_iter()
        .map(|s| creators::ActiveModel {
            name: Set(s),
            ..Default::default()
        })
        .collect();
    let creators_inserted = Creators::insert_many(creators_to_insert)
        .on_conflict(
            OnConflict::column(creators::Column::Name)
                .update_column(creators::Column::Name)
                .to_owned(),
        )
        .exec_with_returning_many(db)
        .await?;
    let creators_relations: Vec<media_creators::ActiveModel> = creators_inserted
        .into_iter()
        .map(|c| media_creators::ActiveModel {
            media_id: Set(id),
            creator_id: Set(c.id),
        })
        .collect();
    dbg!(&creators_relations);
    Ok(MediaCreators::insert_many(creators_relations)
        .on_conflict(
            OnConflict::columns([
                media_creators::Column::MediaId,
                media_creators::Column::CreatorId,
            ])
            .do_nothing()
            .to_owned(),
        )
        .exec_with_returning_many(db)
        .await?)
}

pub async fn creator_delete(
    creators_in: Option<Vec<String>>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    let out = MediaCreators::find()
        .join(JoinType::LeftJoin, media_creators::Relation::Creators.def())
        .filter(creators::Column::Name.is_not_in(creators_in.unwrap_or_else(|| vec![])))
        .filter(media_creators::Column::MediaId.eq(id))
        .all(db)
        .await?;
    MediaCreators::delete_many()
        .filter(
            media_creators::Column::CreatorId
                .is_in(out.iter().map(|c| c.creator_id).collect::<Vec<i64>>()),
        )
        .filter(media_creators::Column::MediaId.eq(id))
        .exec(db)
        .await?;
    dbg!(&out);
    Ok(())
}
