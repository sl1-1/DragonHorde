

pub (crate) const MEDIA_QUERY: &str = r#"
SELECT media.id,
       storage_uri,
       sha256,
       perceptual_hash,
       uploaded,
       media.created,
       title,
       media.description,
       COALESCE(JSON_OBJECT_AGG(t.name, ts) FILTER (WHERE t.media_id = media.id), '{}') tag_groups,
       COALESCE(json_agg(DISTINCT creators.name) FILTER (WHERE media_creators.media_id = media.id), '[]') creators,
       COALESCE(json_agg(DISTINCT collections.name) FILTER (WHERE media_collection.media_id = media.id), '[]') collections,
       COALESCE(json_agg(DISTINCT sources.source) FILTER (WHERE sources.media_id = media.id), '[]') sources
FROM media
         LEFT JOIN (SELECT mt.media_id, tag_groups.name, jsonb_agg(t.tag) as ts
                    FROM tag_groups
                             LEFT JOIN public.tags t on tag_groups.id = t.group
                             LEFT JOIN public.media_tags mt on t.id = mt.tag_id
                    group by tag_groups.name, mt.media_id) as t on media_id = media.id
            LEFT JOIN media_creators on media.id = media_creators.media_id
            LEFT JOIN creators on media_creators.creator_id = creators.id
             LEFT JOIN media_collection on media.id = media_collection.media_id
             LEFT JOIN collections on collections.id = media_collection.collection_id
            LEFT JOIN sources on media.id = sources.media_id
            GROUP BY media.id
"#;

pub(crate) const MEDIA_QUERY_ID: &str = r#"
SELECT media.id,
       storage_uri,
       sha256,
       perceptual_hash,
       uploaded,
       media.created,
       title,
       media.description,
       COALESCE(JSON_OBJECT_AGG(t.name, ts) FILTER (WHERE t.media_id = media.id), '{}') tag_groups,
       COALESCE(json_agg(DISTINCT creators.name) FILTER (WHERE media_creators.media_id = media.id), '[]') creators,
       COALESCE(json_agg(DISTINCT collections.name) FILTER (WHERE media_collection.media_id = media.id), '[]') collections,
       COALESCE(json_agg(DISTINCT sources.source) FILTER (WHERE sources.media_id = media.id), '[]') sources
FROM media
         LEFT JOIN (SELECT mt.media_id, tag_groups.name, jsonb_agg(t.tag) as ts
                    FROM tag_groups
                             LEFT JOIN public.tags t on tag_groups.id = t.group
                             LEFT JOIN public.media_tags mt on t.id = mt.tag_id
                    group by tag_groups.name, mt.media_id) as t on media_id = media.id
            LEFT JOIN media_creators on media.id = media_creators.media_id
            LEFT JOIN creators on media_creators.creator_id = creators.id
             LEFT JOIN media_collection on media.id = media_collection.media_id
             LEFT JOIN collections on collections.id = media_collection.collection_id
            LEFT JOIN sources on media.id = sources.media_id
            WHERE media.id = $1
            GROUP BY media.id
"#;