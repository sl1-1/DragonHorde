use crate::error::AppError;
use crate::AppState;
use axum::extract::State;
use axum::Json;
use axum_extra::extract::Query;
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(IntoParams, Debug, Deserialize)]
pub struct TagQuery {
    tag: String,
}

// #[derive(Debug)]
// pub struct TagResults {
//     results: Vec
// }

#[derive(Copy, Clone, Debug)]
enum QueryAs {
    Tag
}

#[utoipa::path(get, path = "/v1/tags", params(TagQuery), responses((status = OK, body = Vec<String>)), tags = ["tags"])]
pub async fn search_tags(
    state: State<AppState>,
    query: Query<TagQuery>,
) -> Result<Json<Vec<String>>, AppError> {
    let res = sqlx::query_scalar!("SELECT tag from tags
           LEFT JOIN media_tags on tags.id = media_tags.tag_id
           WHERE tag like $1
           GROUP BY tags.tag
            ORDER BY count(media_tags.media_id)
                   ", format!("{}%", query.tag.as_str())).fetch_all(&state.conn).await?;

    Ok(
        Json(res),
    )
}