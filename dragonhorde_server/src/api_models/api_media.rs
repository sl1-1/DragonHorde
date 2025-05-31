use chrono::{DateTime, FixedOffset};

use sea_orm::{
    FromJsonQueryResult, FromQueryResult
};
use serde::{de, Deserialize, Serialize};
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
    #[serde(default)]
    #[schema(value_type = Option<BTreeMap<String, Vec<String>>>)]
    pub tag_groups: Option<DataMap>,
    /// Description of this item, if available
    pub description: Option<String>,
    ///Distance when searching by perceptual hash
    #[schema(read_only)]
    pub distance: Option<f64>,
}

#[skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub result: Vec<ApiMedia>,
}

fn deserialize_perceptual_hash<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let value = Vec::deserialize(deserializer)?;
    Ok(Some(value))
    // // define a visitor that deserializes
    // // `ActualData` encoded as json within a string
    // struct JsonStringVisitor;
    //
    // impl<'de> de::Visitor<'de> for JsonStringVisitor {
    //     type Value = ActualData;
    //
    //     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    //         formatter.write_str("a string containing json data")
    //     }
    //
    //     fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    //     where
    //         E: de::Error,
    //     {
    //         // unfortunately we lose some typed information
    //         // from errors deserializing the json string
    //         serde_json::from_str(v).map_err(E::custom)
    //     }
    // }
    //
    // // use our visitor to deserialize an `ActualValue`
    // deserializer.deserialize_any(JsonStringVisitor)
}