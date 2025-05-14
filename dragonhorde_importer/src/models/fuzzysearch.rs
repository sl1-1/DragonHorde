use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel,  serde::Deserialize, serde::Serialize)]
#[sea_orm(table_name = "fuzzysearch")]
pub struct Model {
    #[sea_orm(primary_key, autoincrement=true)]
    #[serde(skip_deserializing)]
    pub key: i64,
    pub(crate) site: String,
    pub id: i64,
    pub artists: Option<String>,
    #[sea_orm(indexed, nullable)]
    pub(crate) hash: i64,
    pub(crate) posted_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    updated_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    #[sea_orm(indexed, nullable)]
    pub(crate) sha256: Option<String>,
    pub(crate) deleted: bool,
    content_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
