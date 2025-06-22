use sqlx::{Postgres, Transaction};
use crate::error::AppError;

pub async fn creators_create(
    creators_in: Vec<String>,
    db: &mut Transaction<'_, Postgres>,
) -> Result<Vec<i64>, AppError> {
        // Look through existing Creator Aliases, returning id and aliases that exist
        let existing: Vec<(i64, String)> = sqlx::query!(
            r#"SELECT creator, alias from creator_alias WHERE alias = any($1::varchar[])"#,
            &creators_in
                .iter()
                .map(|c| c.to_lowercase())
                .collect::<Vec<String>>()
        )
            .fetch_all(&mut **db)
            .await?
            .into_iter()
            .map(|i| (i.creator, i.alias))
            .collect();

        let (existing_id, existing_name): (Vec<_>, Vec<_>) = existing.into_iter().unzip();

        //Create the models to be inserted that don't exist.
        // Filtering using the results from the first step
        let creators_to_insert: Vec<String> = creators_in
            .into_iter()
            .filter(|c| !existing_name.contains(&c.to_lowercase()))
            .collect();

        let mut creators_inserted: Vec<i64> = Vec::new();
        if creators_to_insert.len() > 0 {
            creators_inserted = sqlx::query_scalar!(
                r#"INSERT into creators (name) SELECT * FROM unnest($1::text[]) RETURNING id"#,
                &creators_to_insert[..]
            )
                .fetch_all(&mut **db)
                .await?;
        }
        //Merge the newly created creators, and the existing creator ids
        creators_inserted.extend(existing_id);
        Ok(creators_inserted)
}