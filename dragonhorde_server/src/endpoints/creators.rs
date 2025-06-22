pub(crate) use crate::api_models::api_creator::{ApiCreator, CreatorsResults};
use crate::api_models::ApiCreatorResult;
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::AppState;
use axum::extract::{Path, State};
use axum::Json;
use sqlx::types::chrono::FixedOffset;
use std::collections::HashSet;

async fn check_creator(id: i64, db: &sqlx::PgPool) -> Result<ApiCreatorResult, AppError> {
    match sqlx::query_file_as!(
        ApiCreatorResult,
        "sql/endpoints/creators/get_creators.sqlx",
        &vec![id][..]
    )
    .fetch_optional(db)
    .await?
    {
        None => Err(NotFound(format!("Creator {} not found", id))),
        Some(c) => Ok(c),
    }
}

#[utoipa::path(get, path = "/v1/creators", responses((status = OK, body = CreatorsResults)), tags = ["creators"])]
pub async fn get_creators(state: State<AppState>) -> Result<Json<CreatorsResults>, AppError> {
    let creators = sqlx::query_file_as!(
        ApiCreatorResult,
        "sql/endpoints/creators/get_creators.sqlx",
        &vec![][..]
    )
    .fetch_all(&state.conn)
    .await?;
    Ok(Json(CreatorsResults { result: creators }))
}

#[utoipa::path(get, path = "/v1/creators/{id}", responses((status = OK, body = ApiCreator)), tags = ["creators"])]
pub async fn get_creators_id(
    state: State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiCreatorResult>, AppError> {
    Ok(Json(
        check_creator(id, &state.conn).await?,
    ))
}

#[utoipa::path(get, path = "/v1/creators/by_alias/{alias}", responses((status = OK, body = ApiCreator)), tags = ["creators"])]
pub async fn get_creators_by_alias(
    state: State<AppState>,
    Path(alias): Path<String>,
) -> Result<Json<ApiCreatorResult>, AppError> {
    match sqlx::query_file_as!(
        ApiCreatorResult,
        "sql/endpoints/creators/get_creators_by_alias.sqlx",
        alias
    )
    .fetch_optional(&state.conn)
    .await?
    {
        None => Err(NotFound(format!("Creator {} not found", alias))),
        Some(c) => Ok(Json(c)),
    }
}

#[utoipa::path(patch, path = "/v1/creators/{id}", request_body = ApiCreator, responses((status = OK, body = ApiCreator)), tags = ["creators"])]
pub async fn patch_creators_id(
    state: State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ApiCreator>,
) -> Result<Json<ApiCreatorResult>, AppError> {
    let creator = check_creator(id, &state.conn).await?;

    let mut tx = state.conn.begin().await?;

    sqlx::query!(
        r#"UPDATE creators SET name=$2 WHERE id = $1"#,
        id,
        payload.name.unwrap_or(creator.name.unwrap())
    )
    .execute(&mut *tx)
    .await?;

    if let Some(aliases) = payload.aliases {
        let current_aliases: HashSet<String> = HashSet::from_iter(creator.aliases.unwrap());
        let new_aliases: HashSet<String> = HashSet::from_iter(aliases.0);
        let to_delete: Vec<String> = current_aliases
            .difference(&new_aliases)
            .map(|i| i.to_string())
            .collect();
        let to_add: Vec<String> = new_aliases
            .difference(&current_aliases)
            .map(|i| i.to_string())
            .collect();
        if !to_delete.is_empty() {
            sqlx::query!(r#"DELETE FROM creator_alias WHERE creator_alias.creator = $1 AND creator_alias.alias = ANY($2)"#, id, &to_delete[..])
                .execute(&mut *tx)
                .await?;
        }
        if !to_add.is_empty() {
            sqlx::query!(
                r#"INSERT INTO creator_alias (creator, alias) VALUES ($1, UNNEST($2::text[]))"#,
                id,
                &to_add[..]
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;

    Ok(Json(
        check_creator(id, &state.conn).await?,
    ))
}
