use crate::endpoints::media::Pagination;
use entity::{collection_creators, collection_creators::Entity as CollectionCreators};
use entity::{collection_tags, collection_tags::Entity as CollectionTags};
use entity::{collections, collections::Entity as Collections};
use entity::{creator_alias, tags, tags::Entity as Tags};
use entity::{creators, creators::Entity as Creators};
use entity::{media_collection, media_collection::Entity as MediaCollection};
use entity::{media_creators, media_creators::Entity as MediaCreators};
use entity::{media_tags, media_tags::Entity as MediaTags};
use entity::{sources, sources::Entity as Sources};
use entity::{tag_groups, tag_groups::Entity as TagGroups};
use sea_orm::Select;
use sea_orm::prelude::Uuid;
use sea_query::extension::postgres::PgExpr;
use sea_query::{
    Alias, Cond, ConditionalStatement, Expr, ExprTrait, Func, JoinType, Order, PgFunc, Query,
    SelectStatement,
};

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

pub fn base_media() -> SelectStatement {
    let tag_query = Query::select()
        .column((MediaTags, media_tags::Column::MediaId))
        .column((TagGroups, tag_groups::Column::Name))
        .expr_as(
            PgFunc::json_agg(Expr::col((Tags, tags::Column::Tag))),
            Alias::new("ts"),
        )
        .from(TagGroups)
        .join(
            JoinType::LeftJoin,
            Tags,
            Expr::col((Tags, tags::Column::Group)).equals((TagGroups, tag_groups::Column::Id)),
        )
        .join(
            JoinType::LeftJoin,
            MediaTags,
            Expr::col((Tags, tags::Column::Id)).equals((MediaTags, media_tags::Column::TagId)),
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
        // .join(
        //     JoinType::LeftJoin,
        //     MediaTags,
        //     Expr::col((MediaTags, media_tags::Column::MediaId))
        //         .equals((Media, media::Column::Id))
        // )
        // .join(
        //     JoinType::LeftJoin,
        //     Tags,
        //     Expr::col((Tags, tags::Column::Id))
        //         .equals((MediaTags, media_tags::Column::TagId))
        // )
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
        .group_by_col((Media, media::Column::Id))
        .group_by_col((Media, media::Column::Uploaded))
        .order_by((Media, media::Column::Uploaded), Order::Desc)
        .take()
}

pub fn media_from_search(mut q: SelectStatement, search: SelectStatement) -> SelectStatement {
    q.cond_where(Cond::all().add(Expr::col((Media, media::Column::Id)).in_subquery(search)))
        .take()
}

pub fn base_search_query() -> SelectStatement {
    Query::select()
        .column((Media, media::Column::Id))
        .from(Media)
        .join(
            JoinType::LeftJoin,
            MediaTags,
            Expr::col((MediaTags, media_tags::Column::MediaId)).equals((Media, media::Column::Id)),
        )
        .join(
            JoinType::LeftJoin,
            Tags,
            Expr::col((Tags, tags::Column::Id)).equals((MediaTags, media_tags::Column::TagId)),
        )
        .join(
            JoinType::LeftJoin,
            MediaCollection,
            Expr::col((MediaCollection, media_collection::Column::MediaId))
                .equals((Media, media::Column::Id)),
        )
        .join(
            JoinType::LeftJoin,
            Collections,
            Expr::col((Collections, collections::Column::Id))
                .equals((MediaCollection, media_collection::Column::CollectionId)),
        )
        // .expr_as(
        //     Expr::cust("COALESCE(json_agg(DISTINCT collections.name) FILTER (WHERE media_collection.media_id = media.id), '[]')"),
        //     Alias::new("collections"))
        .group_by_col((Media, media::Column::Id))
        .take()
}

pub fn search_has_tags(mut q: SelectStatement, has: Vec<String>) -> SelectStatement {
    q.and_having(
        PgFunc::array_agg(Expr::col(tags::Column::Tag))
            .contains(Expr::value(has).cast_as(Alias::new("citext[]"))),
    )
    .take()
}

pub fn search_not_tags(mut q: SelectStatement, has_not: Vec<String>) -> SelectStatement {
    q.and_having(ExprTrait::not(
        PgFunc::array_agg(Expr::col(tags::Column::Tag))
            .contains(Expr::value(has_not).cast_as(Alias::new("citext[]"))),
    ))
    .take()
}

pub fn search_creator(mut q: SelectStatement, creators: Vec<String>) -> SelectStatement {
    q.join(
        JoinType::LeftJoin,
        MediaCreators,
        Expr::col((MediaCreators, media_creators::Column::MediaId))
            .equals((Media, media::Column::Id)),
    )
    .join(
        JoinType::LeftJoin,
        Creators,
        Expr::col((Creators, creators::Column::Id))
            .equals((MediaCreators, media_creators::Column::CreatorId)),
    )
    .and_having(
        Func::lower(Expr::col((Creators, creators::Column::Name))).is_in(
            creators
                .into_iter()
                .map(|s| s.to_lowercase())
                .collect::<Vec<String>>(),
        ),
    )
    .group_by_col((Creators, creators::Column::Name))
    .take()
}

pub fn media_by_creator(mut q: SelectStatement, creator: i64) -> SelectStatement {
    q.join(
        JoinType::LeftJoin,
        MediaCreators,
        Expr::col((MediaCreators, media_creators::Column::MediaId))
            .equals((Media, media::Column::Id)),
    )
    .join(
        JoinType::LeftJoin,
        Creators,
        Expr::col((Creators, creators::Column::Id))
            .equals((MediaCreators, media_creators::Column::CreatorId)),
    )
    .and_having(Expr::col((Creators, creators::Column::Id)).eq(creator))
    .group_by_col((Creators, creators::Column::Id))
    .take()
}

pub fn media_uncollected(mut q: SelectStatement) -> SelectStatement {
    q.and_having(Expr::col((MediaCollection, media_collection::Column::CollectionId)).is_null())
        .group_by_col((MediaCollection, media_collection::Column::CollectionId))
        .take()
}

pub fn media_by_sha(mut q: SelectStatement, sha: &String) -> SelectStatement {
    q.and_having(Expr::col((Media, media::Column::Sha256)).eq(sha))
        .take()
}

pub fn pagination(mut q: SelectStatement, pagination: Pagination) -> SelectStatement {
    let per_page = pagination.per_page.unwrap_or(50);
    let offset = pagination.last.unwrap_or(0);
    q.limit(per_page).offset(offset).take()
}

pub fn media_item(id: i64) -> SelectStatement {
    base_media()
        .and_where(Expr::col((Media, media::Column::Id)).eq(id))
        .take()
}

pub fn base_collection() -> SelectStatement {
    let tag_query = Query::select()
        .column((CollectionTags, collection_tags::Column::CollectionId))
        .column((TagGroups, tag_groups::Column::Name))
        .expr_as(
            PgFunc::json_agg(Expr::col((Tags, tags::Column::Tag))),
            Alias::new("ts"),
        )
        .from(TagGroups)
        .join(
            JoinType::LeftJoin,
            Tags,
            Expr::col((Tags, tags::Column::Group)).equals((TagGroups, tag_groups::Column::Id)),
        )
        .join(
            JoinType::LeftJoin,
            CollectionTags,
            Expr::col((Tags, tags::Column::Id))
                .equals((CollectionTags, collection_tags::Column::TagId)),
        )
        .group_by_col(tag_groups::Column::Name)
        .group_by_col((CollectionTags, collection_tags::Column::CollectionId))
        .take();

    Query::select()
        .column((Collections, collections::Column::Id))
        .column((Collections, collections::Column::Name))
        .column((Collections, collections::Column::Description))
        .column((Collections, collections::Column::Created))
        .from(Collections)
        .join(
            JoinType::LeftJoin,
            CollectionCreators,
            Expr::col((CollectionCreators, collection_creators::Column::CollectionId))
                .equals((Collections, collections::Column::Id))
        )
        .join(
            JoinType::LeftJoin,
            Creators,
            Expr::col((Creators, creators::Column::Id))
                .equals((CollectionCreators, collection_creators::Column::CreatorId))
        )
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT creators.name) FILTER (WHERE collection_creators.collection_id = collections.id), '[]')"),
            Alias::new("creators"))

        .join_subquery(
            JoinType::LeftJoin,
            tag_query.to_owned(),
            Alias::new("t"),
            Expr::cust("t.collection_id = collections.id")
        )
        .expr_as(
            Expr::cust("COALESCE(JSON_OBJECT_AGG(t.name, ts) FILTER (WHERE t.collection_id = collections.id), '{}')"),
            Alias::new("tag_groups")
        )
        .group_by_col((Collections, collections::Column::Id))
        .group_by_col((Collections, collections::Column::Created))
        // .group_by_col((MediaCollection, media_collection::Column::Ord))
        .order_by((Collections, collections::Column::Created), Order::Desc)
        // .order_by((MediaCollection, media_collection::Column::Ord), Order::Asc)
        .take()
}

pub fn collection_with_media(mut q: SelectStatement) -> SelectStatement {
    q.join(
        JoinType::LeftJoin,
        MediaCollection,
        Expr::col((MediaCollection, media_collection::Column::CollectionId))
            .equals((Collections, collections::Column::Id))
    )
        .join(
            JoinType::LeftJoin,
            Media,
            Expr::col((Media, media::Column::Id))
                .equals((MediaCollection, media_collection::Column::MediaId))
        )
        .expr_as(
            Expr::cust("COALESCE(json_agg(media.id ORDER BY media_collection.ord) FILTER (WHERE media_collection.collection_id = collections.id), '[]')"),
            Alias::new("media"))
        .take()
}

pub fn collection(mut q: SelectStatement, id: i64) -> SelectStatement {
    q.and_where(Expr::col((Collections, collections::Column::Id)).eq(id))
        .take()
}

pub fn collections_by_creator(mut q: SelectStatement, creator_id: i64) -> SelectStatement {
    q.and_having(Expr::col((Creators, creators::Column::Id)).eq(creator_id))
        .group_by_col((Creators, creators::Column::Id))
        .take()
}
