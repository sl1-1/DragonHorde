use chrono::{DateTime, FixedOffset};

use crate::api_models::DataVector;

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
#[schema(title="CreatorItem")]
pub struct ApiCreator {
    #[schema(read_only, value_type = i64)]
    pub id: Option<i64>,
    /// date-time that this item was created, if known
    pub created: Option<DateTime<FixedOffset>>,
    pub name: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub aliases: Option<DataVector>,
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
#[schema(title="CreatorItem")]
pub struct ApiCreatorResult {
    #[schema(read_only, value_type = i64)]
    pub id: Option<i64>,
    /// date-time that this item was created, if known
    pub created: Option<DateTime<FixedOffset>>,
    pub name: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    #[serde(default)]
    pub aliases: Option<Vec<String>>,
}

impl Default for ApiCreator {
    fn default() -> Self {
        ApiCreator{
            id: None,
            created: None,
            name: None,
            aliases: None,
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreatorsResults {
    pub result: Vec<ApiCreatorResult>,
}