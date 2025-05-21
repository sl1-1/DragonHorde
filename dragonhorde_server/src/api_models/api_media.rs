use chrono::{DateTime, FixedOffset};

use sea_orm::{
    FromJsonQueryResult, FromQueryResult
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

pub(crate) use crate::api_models::{DataMap, DataVector};

#[skip_serializing_none]
#[derive(
    utoipa::ToSchema,
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    FromQueryResult,
    FromJsonQueryResult,
)]
pub struct ApiMedia {
    #[schema(read_only, value_type = i64)]
    pub id: Option<i64>,
    #[schema(read_only, value_type = String)]
    pub storage_uri: Option<String>,
    #[schema(read_only, value_type = String)]
    pub sha256: Option<String>,
    #[schema(read_only, value_type = String)]
    pub perceptual_hash: Option<String>,
    /// date-time that this item was uploaded
    #[schema(read_only, value_type = DateTime<FixedOffset>)]
    pub uploaded: Option<DateTime<FixedOffset>>,
    /// date-time that this item was created, if known
    pub created: Option<DateTime<FixedOffset>>,
    pub title: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub creators: Option<DataVector>,
    /// Known source locations for this item
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub sources: Option<DataVector>,
    /// Collections this item is in
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub collections: Option<DataVector>,
    #[serde(default)]
    #[schema(value_type = Option<BTreeMap<String, Vec<String>>>)]
    pub tag_groups: Option<DataMap>,
    /// Description of this item, if available
    pub description: Option<String>,
}

#[skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub result: Vec<ApiMedia>,
}