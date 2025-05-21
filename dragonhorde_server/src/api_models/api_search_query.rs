use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, IntoParams, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) creators: Vec<String>,
}