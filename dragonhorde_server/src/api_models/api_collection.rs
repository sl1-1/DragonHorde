use std::collections::{BTreeMap, HashMap};
use crate::api_models::{DataMap, DataVector, DataVectorI64};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[serde_with::skip_serializing_none]
#[derive(
    utoipa::ToSchema,
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[schema(title="CollectionItem")]
pub struct ApiCollection {
    #[schema(read_only, value_type = i64)]
    pub id: Option<i64>,
    /// date-time that this item was created, if known
    pub created: Option<DateTime<FixedOffset>>,
    pub name: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub creators: Option<DataVector>,
    #[serde(default)]
    #[schema(value_type = Option<BTreeMap<String, Vec<String>>>)]
    pub tag_groups: Option<DataMap>,
    /// Description of this item, if available
    pub description: Option<String>,
    #[schema(value_type = Option<Vec<i64>>)]
    #[serde(default)]
    pub media: Option<DataVectorI64>,
    #[schema(no_recursion)]
    #[serde(default)]
    pub children: Option<Vec<ApiCollection>>,
    pub parent: Option<i64>,
}

#[serde_with::skip_serializing_none]
#[derive(
    utoipa::ToSchema,
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[schema(title="CollectionItem")]
pub struct ApiCollectionResult {
    #[schema(read_only, value_type = i64)]
    pub id: Option<i64>,
    /// date-time that this item was created, if known
    pub created: Option<DateTime<FixedOffset>>,
    pub name: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub creators:  Option<Vec<String>>,
    #[serde(default)]
    #[schema(value_type = Option<HashMap<String, Vec<String>>>)]
    pub tag_groups: Option<sqlx::types::Json<HashMap<String, Vec<String>>>>,
    /// Description of this item, if available
    pub description: Option<String>,
    #[schema(value_type = Option<Vec<i64>>)]
    #[serde(default)]
    pub media: Option<Vec<i64>>,
    #[schema(no_recursion, value_type = Option<Vec<ApiCollectionResult>>)]
    #[serde(default)]
    pub children: Option<Vec<sqlx::types::Json<ApiCollectionResult>>>,
    pub parent: Option<i64>,
}


#[serde_with::skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionResult {
    pub result: Vec<ApiCollectionResult>,
}
