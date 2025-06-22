use std::collections::HashMap;
use crate::error::AppError;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;


#[serde_with::skip_serializing_none]
#[derive(
    utoipa::ToSchema,
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[schema(title="DuplicateItem")]
pub struct ApiDuplicate {
    #[schema(read_only, value_type = i64)]
    pub id: i64,
    #[schema(value_type = Option<BTreeMap<i64, i64>>)]
    #[serde(default)]
    pub duplicates: Option<sqlx::types::Json<HashMap<i64, i64>>>,
}

#[serde_with::skip_serializing_none]
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DuplicateResult {
    pub result: Vec<ApiDuplicate>,
}

#[derive(Clone, Debug, IntoParams, Deserialize)]
pub struct DuplicateQuery {
    pub(crate) distance: Option<u64>,
}

#[utoipa::path(get, path = "/v1/duplicates", params(DuplicateQuery), responses((status = OK, body = DuplicateResult)), tags = ["duplicates"])]
pub async fn get_duplicates(
    state: State<AppState>,
    query: axum::extract::Query<DuplicateQuery>,
) -> Result<(StatusCode, Json<DuplicateResult>), AppError> {
    let found_collections = sqlx::query_file_as!(
        ApiDuplicate,
        "sql/endpoints/duplicates/get_duplicates.sqlx",
        query.distance.unwrap_or(1) as f64
    )
        .fetch_all(&state.conn)
        .await?;
    dbg!(&found_collections);
    Ok((
        StatusCode::OK,
        Json(DuplicateResult {
            result: found_collections,
        }),
    ))
}
