use chrono::{DateTime, FixedOffset};

use sea_orm::{
    FromJsonQueryResult, FromQueryResult
};
use serde::{de, Deserialize, Serialize};
use serde_with::skip_serializing_none;

pub(crate) use crate::api_models::{DataMap, DataVector};
use crate::api_models::{DataMapI64String};

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
#[schema(title="MediaItem")]
pub struct ApiMedia {
    #[schema(read_only, value_type = i64)]
    pub id: Option<i64>,
    #[schema(read_only, value_type = String)]
    pub storage_uri: Option<String>,
    #[schema(read_only, value_type = String)]
    pub sha256: Option<String>,
    #[schema(read_only, value_type = i64)]
    // #[serde(deserialize_with = "deserialize_perceptual_hash")]
    pub perceptual_hash: Option<i64>,
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
    /// Collections this item is in
    #[schema(value_type = Option<HashMap<i64, String>>)]
    #[serde(default)]
    pub collections_with_id: Option<DataMapI64String>,
    #[serde(default)]
    #[schema(value_type = Option<BTreeMap<String, Vec<String>>>)]
    pub tag_groups: Option<DataMap>,
    /// Description of this item, if available
    pub description: Option<String>,
    ///Distance when searching by perceptual hash
    #[schema(read_only)]
    pub distance: Option<f64>,
    #[schema(read_only)]
    pub metadata: Option<serde_json::Value>,
    pub file_type: Option<String>,
}

#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
pub struct ImageResolution{
    pub(crate) width: u32,
    pub(crate) height: u32,
}
#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
pub struct ImageMetadata {
    pub(crate) resolution: ImageResolution,
    pub(crate) bits_per_pixel: u16,
    pub(crate) transparent: bool
}