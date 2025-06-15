use entity::{collection_creators, collection_creators::Entity as CollectionCreators};
use entity::{collection_tags, collection_tags::Entity as CollectionTags};
use entity::{collections, collections::Entity as Collections};
use entity::{creators, creators::Entity as Creators};
use entity::{media_collection, media_collection::Entity as MediaCollection};
use entity::{media_creators, media_creators::Entity as MediaCreators};
use entity::{media_tags, media_tags::Entity as MediaTags};
use entity::{sources, sources::Entity as Sources};
use entity::{tag_groups, tag_groups::Entity as TagGroups};
use entity::{tags, tags::Entity as Tags};
use sea_query::extension::postgres::PgExpr;
use sea_query::{
    Alias, Cond, Expr, ExprTrait, Func, JoinType, Order, PgFunc, Query,
    SelectStatement,
};

use crate::api_models::Pagination;
use entity::{media, media::Entity as Media};
use entity::{creator_alias, creator_alias::Entity as CreatorAlias};

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
        // .column(media::Column::PerceptualHash)
        .expr(Expr::col(media::Column::PerceptualHash).cast_as(Alias::new("bigint")))
        .column(media::Column::Uploaded)
        .column((Media, media::Column::Created))
        .column(media::Column::Title)
        .column((Media, media::Column::Description))
        .column((Media, media::Column::Metadata))
        .column((Media, media::Column::Type))
        .expr_as(Expr::col((Media, media::Column::Type)), Alias::new("file_type"))
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
        .expr_as(
            Expr::cust("COALESCE(json_object_agg(DISTINCT collections.id, collections.name) FILTER (WHERE media_collection.media_id = media.id), '{}')"),
            Alias::new("collections_with_id"))
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
        .order_by((Media, media::Column::Uploaded), Order::Desc)
        .take()
}

pub fn search_has_tags(mut q: SelectStatement, has: Vec<String>) -> SelectStatement {
    q.and_having(
        PgFunc::array_agg(Expr::col(tags::Column::Tag))
            .contains(Expr::value(has)),
    )
    .take()
}

pub fn search_not_tags(mut q: SelectStatement, has_not: Vec<String>) -> SelectStatement {
    q.and_having(ExprTrait::not(
        PgFunc::array_agg(Expr::col(tags::Column::Tag))
            .contains(Expr::value(has_not)),
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
    let media_query = Query::select()
        .column((MediaCollection, media_collection::Column::CollectionId))
        .column((MediaCollection, media_collection::Column::MediaId))
        .column((MediaCollection, media_collection::Column::Ord))
        .from(MediaCollection)
        .order_by(media_collection::Column::Ord,  Order::Asc)
        .take();
    q.join_subquery(JoinType::LeftJoin, media_query, Alias::new("media_collection"), Expr::cust("media_collection.collection_id = collections.id"))
    // q.join(
    //     JoinType::LeftJoin,
    //     MediaCollection,
    //     Expr::col((MediaCollection, media_collection::Column::CollectionId))
    //         .equals((Collections, collections::Column::Id))
    // )
        .join(
            JoinType::LeftJoin,
            Media,
            Expr::col((Media, media::Column::Id))
                .equals((MediaCollection, media_collection::Column::MediaId))
        )
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT media.id) FILTER (WHERE media_collection.collection_id = collections.id), '[]')"),
            Alias::new("media"))
        .take()
}

pub fn collection_with_children(mut q: SelectStatement) -> SelectStatement {
    let child_query = Query::select()
        .column((Collections, collections::Column::Id))
        .column((Collections, collections::Column::Name))
        .column((Collections, collections::Column::Description))
        .column((Collections, collections::Column::Created))
        .column((Collections, collections::Column::Parent))
        .from(Collections)
        .take();
        // .and_where(Expr::col((Collections, collections::Column::Parent)).eq(id))

    q.join_subquery(JoinType::LeftJoin, child_query.to_owned(), Alias::new("children"),  Expr::cust("children.parent = collections.id"))
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT children)  FILTER (WHERE children.parent = collections.id), '[]')"),
            Alias::new("children"))
        .take()
}

pub fn collection(mut q: SelectStatement, id: i64) -> SelectStatement {
    q.and_where(Expr::col((Collections, collections::Column::Id)).eq(id))
        .take()
}

pub fn collection_by_name(mut q: SelectStatement, name: &String) -> SelectStatement {
    q.and_where(Expr::col((Collections, collections::Column::Name)).eq(name))
        .and_having(Expr::col((Collections, collections::Column::Parent)).is_null())
        .take()

}

pub fn collection_by_path(path: String) -> Statement {
    let path_split:Vec<String> = path.split('/').map(|s| s.to_string()).collect();

    Statement::from_sql_and_values(DatabaseBackend::Postgres, r#"
SELECT "collections"."id",
       "collections"."name",
       "collections"."description",
       "collections"."created",
       COALESCE(json_agg(DISTINCT creators.name) FILTER (WHERE collection_creators.collection_id = collections.id),
                '[]')                                                                              AS "creators",
       COALESCE(JSON_OBJECT_AGG(t.name, ts) FILTER (WHERE t.collection_id = collections.id), '{}') AS "tag_groups",
       COALESCE(json_agg(media.id ORDER BY media_collection.ord)
                FILTER (WHERE media_collection.collection_id = collections.id), '[]')              AS "media",
       COALESCE(json_agg(DISTINCT children) FILTER (WHERE children.parent = collections.id), '[]')          AS "children"
FROM "collections"
         LEFT JOIN "collection_creators" ON "collection_creators"."collection_id" = "collections"."id"
         LEFT JOIN "creators" ON "creators"."id" = "collection_creators"."creator_id"
         LEFT JOIN (SELECT "collection_tags"."collection_id", "tag_groups"."name", JSON_AGG("tags"."tag") AS "ts"
                    FROM "tag_groups"
                             LEFT JOIN "tags" ON "tags"."group" = "tag_groups"."id"
                             LEFT JOIN "collection_tags" ON "tags"."id" = "collection_tags"."tag_id"
                    GROUP BY "name", "collection_tags"."collection_id") AS "t" ON t.collection_id = collections.id
         LEFT JOIN "media_collection" ON "media_collection"."collection_id" = "collections"."id"
         LEFT JOIN "media" ON "media"."id" = "media_collection"."media_id"
         LEFT JOIN (SELECT "collections"."id",
                           "collections"."name",
                           "collections"."description",
                           "collections"."created",
                           "collections"."parent"
                    FROM "collections") AS "children" ON children.parent = collections.id
WHERE "collections"."id" = (
    WITH RECURSIVE cte AS (
        SELECT id, name, parent, array[name] as path
        FROM collections
        UNION ALL
        SELECT c.id, c.name, c.parent, ct.path || c.name
        FROM cte ct JOIN
             collections c
             ON c.parent = ct.id
    )
    SELECT id
    FROM cte
    WHERE path = $1::varchar[]
    )
GROUP BY "collections"."id", "collections"."created"
ORDER BY "collections"."created" DESC
    "#, [path_split.into()])
}

pub fn collection_id_by_path(path: String) -> Statement {
    let path_split:Vec<String> = path.split('/').map(|s| s.to_string()).collect();

    Statement::from_sql_and_values(DatabaseBackend::Postgres, r#"
        WITH RECURSIVE cte AS (
        SELECT id, name, parent, array[name] as path
        FROM collections
        UNION ALL
        SELECT c.id, c.name, c.parent, ct.path || c.name
        FROM cte ct JOIN
             collections c
             ON c.parent = ct.id
    )
    SELECT id
    FROM cte
    WHERE path = $1::varchar[]
    "#, [path_split.into()])
}

pub fn collections_by_creator(mut q: SelectStatement, creator_id: i64) -> SelectStatement {
    q.and_having(Expr::col((Creators, creators::Column::Id)).eq(creator_id))
    .and_having(Expr::col((Collections, collections::Column::Parent)).is_null())
        .group_by_col((Creators, creators::Column::Id))
        .take()
}

pub fn base_creator() -> SelectStatement {
    Query::select()
        .from(Creators)
        .column((Creators, creators::Column::Id))
        .column((Creators, creators::Column::Name))
        .column((Creators, creators::Column::Created))
        .join(
            JoinType::LeftJoin,
            CreatorAlias,
            Expr::col((CreatorAlias, creator_alias::Column::Creator))
                .equals((Creators, creators::Column::Id))
        )
        .expr_as(
            Expr::cust("COALESCE(json_agg(DISTINCT creator_alias.alias) FILTER (WHERE creator_alias.creator = creators.id), '[]')"),
            Alias::new("aliases"))
        .group_by_col((Creators, creators::Column::Id))
        .order_by((Creators, creators::Column::Name), Order::Asc)
        .take()
}

pub fn creator_by_id(mut q: SelectStatement, id: i64) -> SelectStatement {
    q.and_where(Expr::col((Creators, creators::Column::Id)).eq(id)).take()
}

pub fn creator_by_alias(mut q: SelectStatement, alias: &str) -> SelectStatement {
    let alias_select = Query::select()
        .from(CreatorAlias)
        .column((CreatorAlias, creator_alias::Column::Creator))
        .and_where(Expr::col((CreatorAlias, creator_alias::Column::Alias)).eq(alias))
        .take();
    q.cond_where(Cond::all().add(Expr::col((Creators, creators::Column::Id)).in_subquery(alias_select)))
        .take()
}


pub fn search_hash(mut q: SelectStatement, hash: i64, distance: Option<i64>) -> SelectStatement {
    Expr::cust_with_expr("perceptual_hash <~> $1::bit(64)", hash);
        q.and_where(Expr::cust_with_expr("perceptual_hash <~> $1::bit(64)", hash).lt(distance.unwrap_or(10)))
            .order_by_expr(Expr::cust_with_expr("perceptual_hash <~> $1::bit(64)", hash), Order::Asc)
        .take()
}


pub fn distance(mut q: SelectStatement, hash: i64) -> SelectStatement {
    let a = Alias::new("distance");
    q.expr_as(Expr::cust_with_expr("perceptual_hash <~> $1::bit(64)", hash), a.clone())
        .clear_order_by()
        .order_by_expr(Expr::cust_with_expr("perceptual_hash <~> $1::bit(64)", hash), Order::Asc)
        .take()
}