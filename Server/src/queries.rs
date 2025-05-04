use sea_query::{Alias, Expr, ExprTrait, JoinType, Order, PgFunc, Query, SelectStatement};
use sea_query::extension::postgres::PgExpr;
use crate::endpoints::media::Pagination;
use entity::{tags, tags::Entity as Tags};
use entity::{tag_groups, tag_groups::Entity as TagGroups};
use entity::{sources, sources::Entity as Sources};
use entity::{creators, creators::Entity as Creators};
use entity::{media_creators, media_creators::Entity as MediaCreators};
use entity::{media_tags, media_tags::Entity as MediaTags};
use entity::{collections, collections::Entity as Collections};
use entity::{media_collection, media_collection::Entity as MediaCollection};
use entity::{media, media::Entity as Media};



// pub (crate) const MEDIA_QUERY: &str = r#"
// SELECT media.id,
//        storage_uri,
//        sha256,
//        perceptual_hash,
//        uploaded,
//        media.created,
//        title,
//        media.description,
//        COALESCE(JSON_OBJECT_AGG(t.name, ts) FILTER (WHERE t.media_id = media.id), '{}') tag_groups,
//        COALESCE(json_agg(DISTINCT creators.name) FILTER (WHERE media_creators.media_id = media.id), '[]') creators,
//        COALESCE(json_agg(DISTINCT collections.name) FILTER (WHERE media_collection.media_id = media.id), '[]') collections,
//        COALESCE(json_agg(DISTINCT sources.source) FILTER (WHERE sources.media_id = media.id), '[]') sources
// FROM media
//          LEFT JOIN (SELECT mt.media_id, tag_groups.name, jsonb_agg(t.tag) as ts
//                     FROM tag_groups
//                              LEFT JOIN public.tags t on tag_groups.id = t.group
//                              LEFT JOIN public.media_tags mt on t.id = mt.tag_id
//                     group by tag_groups.name, mt.media_id) as t on media_id = media.id
//             LEFT JOIN media_creators on media.id = media_creators.media_id
//             LEFT JOIN creators on media_creators.creator_id = creators.id
//              LEFT JOIN media_collection on media.id = media_collection.media_id
//              LEFT JOIN collections on collections.id = media_collection.collection_id
//             LEFT JOIN sources on media.id = sources.media_id
//             GROUP BY media.id
//             LIMIT $1 OFFSET $2
// "#;
// 
// pub(crate) const MEDIA_QUERY_ID: &str = r#"
// SELECT media.id,
//        storage_uri,
//        sha256,
//        perceptual_hash,
//        uploaded,
//        media.created,
//        title,
//        media.description,
//        COALESCE(JSON_OBJECT_AGG(t.name, ts) FILTER (WHERE t.media_id = media.id), '{}') tag_groups,
//        COALESCE(json_agg(DISTINCT creators.name) FILTER (WHERE media_creators.media_id = media.id), '[]') creators,
//        COALESCE(json_agg(DISTINCT collections.name) FILTER (WHERE media_collection.media_id = media.id), '[]') collections,
//        COALESCE(json_agg(DISTINCT sources.source) FILTER (WHERE sources.media_id = media.id), '[]') sources
// FROM media
//          LEFT JOIN (SELECT mt.media_id, tag_groups.name, jsonb_agg(t.tag) as ts
//                     FROM tag_groups
//                              LEFT JOIN public.tags t on tag_groups.id = t.group
//                              LEFT JOIN public.media_tags mt on t.id = mt.tag_id
//                     group by tag_groups.name, mt.media_id) as t on media_id = media.id
//             LEFT JOIN media_creators on media.id = media_creators.media_id
//             LEFT JOIN creators on media_creators.creator_id = creators.id
//              LEFT JOIN media_collection on media.id = media_collection.media_id
//              LEFT JOIN collections on collections.id = media_collection.collection_id
//             LEFT JOIN sources on media.id = sources.media_id
//             WHERE media.id = $1
//             GROUP BY media.id
// "#;

pub fn search_query(has: Option<Vec<String>>, has_not: Option<Vec<String>>, pagination: Option<Pagination>) -> SelectStatement {
    let tag_query = Query::select()
        .column((MediaTags, media_tags::Column::MediaId))
        .column((TagGroups, tag_groups::Column::Name))
        .expr_as(
            PgFunc::json_agg(Expr::col((Tags, tags::Column::Tag))),
            Alias::new("ts")
        )
        .from(TagGroups)
        .join(
            JoinType::LeftJoin,
            Tags,
            Expr::col((Tags, tags::Column::Group))
                .equals((TagGroups, tag_groups::Column::Id))
        )
        .join(
            JoinType::LeftJoin,
            MediaTags,
            Expr::col((Tags, tags::Column::Id))
                .equals((MediaTags, media_tags::Column::TagId))
        )
        .group_by_col(tag_groups::Column::Name)
        .group_by_col((MediaTags, media_tags::Column::MediaId))
        .take();

    Query::select()
        .column((Media, media::Column::Id))
        .column(media::Column::StorageUri)
        .column(media::Column::Sha256)
        .column(media::Column::PerceptualHash)
        .column(media::Column::Uploaded)
        .column((Media, media::Column::Created))
        .column(media::Column::Title)
        .column((Media, media::Column::Description))
        .from(Media)
        .join(
            JoinType::Join,
            MediaTags,
            Expr::col((MediaTags, media_tags::Column::MediaId))
                .equals((Media, media::Column::Id))
        )
        .join(
            JoinType::Join,
            Tags,
            Expr::col((Tags, tags::Column::Id))
                .equals((MediaTags, media_tags::Column::TagId))
        )
        .join(
            JoinType::LeftJoin,
            MediaCreators,
            Expr::col((MediaCreators, media_creators::Column::MediaId))
                .equals((Media, media::Column::Id))
        )
        .join(
            JoinType::LeftJoin,
            Creators,
            Expr::col((Creators, creators::Column::Id))
                .equals((MediaCreators, media_creators::Column::CreatorId))
        )
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT creators.name) FILTER (WHERE media_creators.media_id = media.id), '[]')"),
            Alias::new("creators"))
        .join(
            JoinType::LeftJoin,
            MediaCollection,
            Expr::col((MediaCollection, media_collection::Column::MediaId))
                .equals((Media, media::Column::Id))
        )
        .join(
            JoinType::LeftJoin,
            Collections,
            Expr::col((Collections, collections::Column::Id))
                .equals((MediaCollection, media_collection::Column::CollectionId))
        )
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT collections.name) FILTER (WHERE media_collection.media_id = media.id), '[]')"),
            Alias::new("collections"))
        .join(
            JoinType::LeftJoin,
           Sources,
            Expr::col((Sources,sources::Column::MediaId))
                .equals((Media, media::Column::Id))
        )
        .expr_as(
            Expr::cust(" COALESCE(json_agg(DISTINCT sources.source) FILTER (WHERE sources.media_id = media.id), '[]')"),
            Alias::new("sources")
        )
        .join_subquery(
            JoinType::LeftJoin,
            tag_query.to_owned(),
            Alias::new("t"),
            Expr::cust("t.media_id = media.id")
        )
        .expr_as(
            Expr::cust("COALESCE(JSON_OBJECT_AGG(t.name, ts) FILTER (WHERE t.media_id = media.id), '{}')"),
            Alias::new("tag_groups")
        )
        .conditions(
            has.is_some(),
            |q| {
                q.and_having(
                    PgFunc::array_agg(Expr::col(tags::Column::Tag))
                        .contains(Expr::value(has.unwrap()).cast_as(Alias::new("citext[]")))
                );
            },
            |_q| {}
        )
        .conditions(
            has_not.is_some(),
            |q| {
                q.and_having(
                    ExprTrait::not(
                        PgFunc::array_agg(Expr::col(tags::Column::Tag))
                            .contains(
                                Expr::value(has_not.unwrap()).cast_as(Alias::new("citext[]")))
                    )
                );
            },
            |_q| {}
        )
        .conditions(
            pagination.is_some(),
            |q| { 
                let pagination = pagination.unwrap();
                let per_page = pagination.per_page.unwrap_or(50);
                let offset = pagination.last.unwrap_or(0);
                q.limit(per_page).offset(offset); 
            },
            |q| { q.limit(50); }
        )
        .group_by_col((Media, media::Column::Id))
        .group_by_col((Media, media::Column::Uploaded))
        .order_by((Media, media::Column::Uploaded), Order::Desc)
        .take()
}

pub fn media_item_query(id: i64) -> SelectStatement{
    search_query(None, None, None)
        .and_where(Expr::col((Media, media::Column::Id))
            .eq(id))
        .take()
}