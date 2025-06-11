use crate::error::AppError;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Query;
use entity::{creators, creators::Entity as Creators, media_collection};
use entity::{collections, collections::Entity as Collections};
use entity::media_creators;
use entity::media_tags;
use entity::{tags, tags::Entity as Tags};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait};
use sea_query::{JoinType, Order};
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use utoipa::IntoParams;

#[derive(utoipa::ToSchema, Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
enum TagType {
    All,
    Tag,
    Artist,
    Collection,
}

impl Default for TagType {
    fn default() -> Self {Self::All}
}

// impl ComposeSchema for TagType {
//     fn compose(new_generics: Vec<RefOr<Schema>>) -> RefOr<Schema> {
//         utoipa::openapi::schema::Object::builder()
//             .schema_type(utoipa::openapi::schema::SchemaType::new(
//                 utoipa::openapi::schema::Type::String,
//             ))
//             .enum_values(Some(["All", "Tag", "Artist", "Collection"]))
//             .into()
//     }
// }

#[derive(IntoParams, Debug, Deserialize)]
pub struct TagQuery {
    tag: String,
    #[param(inline)]
    #[serde(default)]
    tag_type: TagType
}

#[derive(utoipa::ToSchema, Debug, Deserialize, Serialize)]
pub struct TagReturn {
    id: i64,
    tag: String,
    #[schema(inline, example=TagType::Tag)]
    tag_type: TagType
}

#[utoipa::path(get, path = "/v1/autocomplete", params(TagQuery), responses((status = OK, body = Vec<TagReturn>)), tags = ["tags"])]


pub async fn autocomplete(
    state: State<AppState>,
    query: Query<TagQuery>,
) -> Result<(StatusCode, Json<Vec<TagReturn>>), AppError> {
    dbg!(&query);
    let mut combined : Vec<(TagReturn, i64)> = Vec::new();

    let tag = match query.tag.split_once(":") {
        Some(t) => {t.1},
        None => query.tag.as_str(),
    };

    if query.tag_type == TagType::All || query.tag_type == TagType::Artist {
        let creators: Vec<(String, i64)> = Creators::find()
            .select_only()
            .column(creators::Column::Name)
            .column_as(media_creators::Column::MediaId.count(), "count")
            .filter(creators::Column::Name.starts_with(tag))
            .join(JoinType::LeftJoin, creators::Relation::MediaCreators.def())
            .order_by(media_creators::Column::MediaId.count(), Order::Desc)
            .group_by(creators::Column::Name)
            .into_tuple()
            .all(&state.conn).await?;
        combined.extend(creators.into_iter().map(|(creator, count)| (TagReturn{id: 0, tag: creator, tag_type: TagType::Artist}, count)));
    }

    if query.tag_type == TagType::All || query.tag_type == TagType::Collection {
        let collections: Vec<(i64, String, i64)> = Collections::find()
            .select_only()
            .column(collections::Column::Id)
            .column(collections::Column::Name)
            .column_as(media_collection::Column::MediaId.count(), "count")
            .filter(collections::Column::Name.starts_with(tag))
            .join(JoinType::LeftJoin, collections::Relation::MediaCollection.def())
            .order_by(media_collection::Column::MediaId.count(), Order::Desc)
            .group_by(collections::Column::Id)
            .group_by(collections::Column::Name)
            .into_tuple()
            .all(&state.conn).await?;
        combined.extend(collections.into_iter().map(|(id, collection, count)| (TagReturn{id, tag: collection, tag_type: TagType::Collection}, count)));
    }

    if query.tag_type == TagType::All || query.tag_type == TagType::Tag {
        let mut search = tag.to_string();
        let mut neg = false;
        if search.starts_with("-"){
            search = search.replacen("-", "", 1);
            neg = true;
        }
        let tags: Vec<(String, i64)> = Tags::find()
            .select_only()
            .column(tags::Column::Tag)
            .column_as(media_tags::Column::MediaId.count(),  "count")
            .filter(tags::Column::Tag.starts_with(search))
            .join(JoinType::LeftJoin, tags::Relation::MediaTags.def())
            .order_by(media_tags::Column::MediaId.count(), Order::Desc)
            .group_by(tags::Column::Tag)
            .into_tuple()
            .all(&state.conn).await?;
        combined.extend(tags.into_iter().map(|(t, count)| (TagReturn{id:0, tag: if neg {format!("-{}", t)} else {t}, tag_type:TagType::Tag}, count)));
    }



    combined.sort_by_key(|x| x.1);
    combined.reverse();
    // dbg!(&combined);
    Ok((
        StatusCode::OK,
        Json(combined.into_iter().map(|x| x.0).collect()),
    ))
}