use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, IntoParams, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) creators: Vec<String>,
}

#[derive(Debug, IntoParams, Deserialize)]
pub struct HashQuery {
    #[serde(default)]
    pub(crate) hash: i64,
    #[serde(default)]
    pub(crate) max_distance: Option<i64>,
}