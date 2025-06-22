use crate::error::AppError;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Query;
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
    tag_type: TagType,
    tag_group: Option<String>
}

#[utoipa::path(get, path = "/v1/autocomplete", params(TagQuery), responses((status = OK, body = Vec<TagReturn>)), tags = ["tags"])]


pub async fn autocomplete(
    state: State<AppState>,
    query: Query<TagQuery>,
) -> Result<(StatusCode, Json<Vec<TagReturn>>), AppError> {
    dbg!(&query);
    let mut combined : Vec<(TagReturn, i64)> = Vec::new();

    let tag = match query.tag.split_once(":") {
        Some(t) => {t.1.to_string()},
        None => query.tag.as_str().to_string(),
    };

    if query.tag_type == TagType::All || query.tag_type == TagType::Artist {
        let creators: Vec<(String, i64)> = sqlx::query!("SELECT creators.name, count(media_creators.media_id) as count from creators
           LEFT JOIN media_creators ON creators.id = media_creators.creator_id
           WHERE name like $1
           GROUP BY creators.name
            ORDER BY count(media_creators.media_id)
                   ", format!("{}%", tag))
            .fetch_all(&state.conn)
            .await?
            .into_iter().map(|i| (i.name, i.count.unwrap())).collect();
        combined.extend(creators.into_iter().map(|(creator, count)| (TagReturn{id: 0, tag: creator, tag_type: TagType::Artist, tag_group: None}, count)));
    }

    if query.tag_type == TagType::All || query.tag_type == TagType::Collection {
        let collections: Vec<(i64, String, i64)> = sqlx::query!("SELECT collections.id, collections.name, count(media_collection.media_id) as count from collections
           LEFT JOIN media_collection ON collections.id = media_collection.collection_id
           WHERE name like $1
           GROUP BY collections.id, collections.name
            ORDER BY count(media_collection.media_id)
                   ", format!("{}%", tag))
            .fetch_all(&state.conn)
            .await?
            .into_iter().map(|i| (i.id, i.name, i.count.unwrap())).collect();
        combined.extend(collections.into_iter().map(|(id, collection, count)| (TagReturn{id, tag: collection, tag_type: TagType::Collection, tag_group: None}, count)));
    }

    if query.tag_type == TagType::All || query.tag_type == TagType::Tag {
        let mut search = tag.to_string();
        let mut neg = false;
        if search.starts_with("-"){
            search = search.replacen("-", "", 1);
            neg = true;
        }
        let tags: Vec<(String, String, i64)> = sqlx::query!("SELECT tag, name, count(media_tags.media_id) as count from tags
           LEFT JOIN media_tags on tags.id = media_tags.tag_id
           LEFT JOIN tag_groups ON tags.group = tag_groups.id
           WHERE tag like $1
           GROUP BY tags.tag, tag_groups.name
            ORDER BY count(media_tags.media_id)
                   ", format!("{}%", query.tag.as_str()))
            .fetch_all(&state.conn)
            .await?
            .into_iter().map(|i| (i.tag, i.name, i.count.unwrap())).collect();
        combined.extend(tags.into_iter().map(|(t,g, count)| (TagReturn{id:0, tag: if neg {format!("-{}", t)} else {t}, tag_type:TagType::Tag, tag_group: Some(g)}, count)));
    }



    combined.sort_by_key(|x| x.1);
    combined.reverse();
    // dbg!(&combined);
    Ok((
        StatusCode::OK,
        Json(combined.into_iter().map(|x| x.0).collect()),
    ))
}