use super::{configuration, ContentType, Error, ResponseContent};
use crate::models;
use async_trait::async_trait;
use reqwest;
use serde::{de::Error as _, Deserialize, Serialize};
use std::sync::Arc;

#[async_trait]
pub trait SearchApi: Send + Sync {
    async fn search_get(
        &self,
        has_tags: Option<Vec<String>>,
        not_tags: Option<Vec<String>>,
        last: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<models::media::SearchResult, Error<SearchGetError>>;
}

pub struct SearchApiClient {
    configuration: Arc<configuration::Configuration>,
}

impl SearchApiClient {
    pub fn new(configuration: Arc<configuration::Configuration>) -> Self {
        Self { configuration }
    }
}

#[async_trait]
impl SearchApi for SearchApiClient {
    async fn search_get(
        &self,
        has_tags: Option<Vec<String>>,
        not_tags: Option<Vec<String>>,
        last: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<models::media::SearchResult, Error<SearchGetError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/search", local_var_configuration.base_path);
        let mut local_var_req_builder =
            local_var_client.request(reqwest::Method::GET, local_var_uri_str.as_str());

        let mut query_pairs: Vec<(&str, String)> = Vec::new();
        if last.is_some() {
            query_pairs.push(("last", last.unwrap().to_string()));
        }
        if per_page.is_some() {
            query_pairs.push(("per_page", per_page.unwrap().to_string()));
        }
        if let Some(has_tags) = has_tags{
            for tag in has_tags {
                query_pairs.push(("has_tags", tag));
            }
        }
        if let Some(not_tags) = not_tags{
            for tag in not_tags {
                query_pairs.push(("not_tags", tag));
            }
        }

        local_var_req_builder = local_var_req_builder.query(&query_pairs);
        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder
                .header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }

        let local_var_req = local_var_req_builder.build()?;
        let local_var_resp = local_var_client.execute(local_var_req).await?;

        let local_var_status = local_var_resp.status();
        let local_var_content_type = local_var_resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream");
        let local_var_content_type = super::ContentType::from(local_var_content_type);
        let local_var_content = local_var_resp.text().await?;

        if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
            match local_var_content_type {
                ContentType::Json => serde_json::from_str(&local_var_content).map_err(Error::from),
                ContentType::Text => {
                    return Err(Error::from(serde_json::Error::custom(
                        "Received `text/plain` content type response that cannot be converted to `Vec&lt;models::Media&gt;`",
                    )));
                }
                ContentType::Binary => Err(Error::from(serde_json::Error::custom(
                    "Received `application/octet` content type response that cannot be converted to `Vec&lt;models::Media&gt;`",
                ))),
                ContentType::Unsupported(local_var_unknown_type) => {
                    return Err(Error::from(serde_json::Error::custom(format!(
                        "Received `{local_var_unknown_type}` content type response that cannot be converted to `Vec&lt;models::Media&gt;`"
                    ))));
                }
            }
        } else {
            let local_var_entity: Option<SearchGetError> =
                serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent {
                status: local_var_status,
                content: local_var_content,
                entity: local_var_entity,
            };
            Err(Error::ResponseError(local_var_error))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SearchGetError {
    UnknownValue(serde_json::Value),
}
