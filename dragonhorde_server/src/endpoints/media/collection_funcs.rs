use crate::error::AppError;
use entity::{collections, collections::Entity as Collections, media_collection, media_collection::Entity as MediaCollection};
use sea_orm::{
    ColumnTrait, DatabaseTransaction, EntityTrait, JoinType, QuerySelect, RelationTrait, Set,
};
use sea_orm::{DeriveColumn, EnumIter, QueryFilter, QueryOrder, SelectColumns};
use sea_query::Order;

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryAs {
    Ord,
}

pub async fn collections_insert(
    collections_in: &Vec<String>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    for collection in collections_in {
        println!("{:?} {:?}", &id, &collection);
        match Collections::find()
            .join(JoinType::LeftJoin, collections::Relation::MediaCollection.def())
            .filter(collections::Column::Name.eq(collection))
            .order_by(media_collection::Column::Ord, Order::Desc)
            .select_only()
            .column(collections::Column::Id)
            .column(media_collection::Column::Ord)
            .into_tuple::<(i64, Option<i32>)>()
            .one(db)
            .await?
        {
            Some(v) => {
                MediaCollection::insert(media_collection::ActiveModel {
                    media_id: Set(id),
                    collection_id: Set(v.0),
                    ord: Set(Some(v.1.unwrap_or(0)+1)),
                }).exec(db).await?;
            }
            None => {}
        }
    }
    Ok(())
}

pub async fn collections_delete(
    collections_in: &Vec<String>,
    id: i64,
    db: &DatabaseTransaction,
) -> Result<(), AppError> {
    let collections = Collections::find()
        .join(JoinType::LeftJoin, collections::Relation::MediaCollection.def())
        .filter(collections::Column::Name.is_in(collections_in))
        .select_only()
        .select_column(collections::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    MediaCollection::delete_many()
        .filter(media_collection::Column::MediaId.eq(id))
        .filter(media_collection::Column::CollectionId.is_not_in(collections))
        .exec(db)
        .await?;
    Ok(())
}
