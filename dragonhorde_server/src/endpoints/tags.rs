use crate::error::AppError;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Query;
use entity::media_tags;
use entity::{tags, tags::Entity as Tags};
use sea_orm::{ColumnTrait, DeriveColumn, EntityTrait, EnumIter, QueryFilter, QueryOrder, QuerySelect, RelationTrait};
use sea_query::{JoinType, Order};
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryAs {
    Tag
}

#[utoipa::path(get, path = "/v1/tags", params(TagQuery), responses((status = OK, body = Vec<String>)), tags = ["tags"])]
pub async fn search_tags(
    state: State<AppState>,
    query: Query<TagQuery>,
) -> Result<(StatusCode, Json<Vec<String>>), AppError> {
    dbg!(&query);
    let res = Tags::find()
        .select_only()
        .column_as(tags::Column::Tag, QueryAs::Tag)
        .filter(tags::Column::Tag.starts_with(query.tag.as_str()))
        .join(JoinType::LeftJoin, tags::Relation::MediaTags.def())
        .order_by(media_tags::Column::MediaId.count(), Order::Desc)
        .group_by(tags::Column::Tag)
        .into_values::<String, QueryAs>()
        .all(&state.conn).await?;

    Ok((
        StatusCode::OK,
        Json(res),
    ))
}