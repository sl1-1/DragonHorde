use crate::error::AppError;
use entity::{sources, sources::Entity as Sources};
use sea_orm::sea_query::OnConflict;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, Set};

pub async fn sources_insert(sources_in: Vec<String>, id: i64, db: &DatabaseTransaction) -> Result<Vec<sources::Model>, AppError>{
    if !sources_in.is_empty() {
        let source_relations: Vec<sources::ActiveModel> = sources_in
            .into_iter()
            .map(|s| sources::ActiveModel {
                media_id: Set(id),
                source: Set(s),
                ..Default::default()
            })
            .collect();
        Ok(Sources::insert_many(source_relations)
            .on_conflict(
                OnConflict::columns([sources::Column::MediaId, sources::Column::Source])
                    .do_nothing()
                    .to_owned(),
            )
            .exec_with_returning_many(db)
            .await?)
    } else {
        Ok(Vec::new())
    }
}

pub async fn sources_delete(sources_in: Vec<String>, id: i64, db: &DatabaseTransaction) -> Result<(), AppError>{
    Sources::delete_many()
        .filter(sources::Column::Source.is_not_in(sources_in))
        .filter(sources::Column::MediaId.eq(id))
        .exec(db)
        .await?;
    Ok(())
}
