use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use serde_with::skip_serializing_none;
use crate::api_models::{ApiCollection, ApiMedia};

#[derive(Debug, IntoParams, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) creators: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryType {
    All,
    Media,
    Collection,
}

impl Default for QueryType {
    fn default() -> Self {QueryType::All}
}

#[derive(Clone, Debug, PartialEq, Deserialize, utoipa::ToSchema)]
pub struct SearchQueryJson {
    ///Tags to search within. Tags prefixed with - will be excluded
    pub(crate) tags: Option<Vec<String>>,
    // #[serde(default)]
    ///Creators to search within. Creators prefixed with - will be excluded.
    /// If not included in the request, it will query for results that do not have a creator
    pub(crate) creators: Option<Vec<String>>,
    ///Collections to search within. Collections prefixed with - will be excluded.
    /// If not included in the request, it will query for results that do not have a collection
    pub(crate) collections: Option<Vec<String>>,
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) query_type: QueryType,
}

#[derive(Debug, IntoParams, Deserialize)]
pub struct HashQuery {
    #[serde(default)]
    pub(crate) hash: i64,
    #[serde(default)]
    pub(crate) max_distance: Option<i64>,
}

#[skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub result: Vec<ApiMedia>,
    pub collections: Option<Vec<ApiCollection>>,
}

impl Default for SearchResult {
    fn default() -> Self {Self { result: vec![], collections: None }}
}