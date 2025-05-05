use super::{configuration, ContentType, Error, ResponseContent};
use async_trait::async_trait;
use reqwest;
use serde::{de::Error as _, Deserialize, Serialize};
use std::sync::Arc;

#[async_trait]
pub trait TagApi: Send + Sync {
    async fn tag_get(
        &self,
        tag: String,
    ) -> Result<Vec<String>, Error<TagGetError>>;
}

pub struct TagApiClient {
    configuration: Arc<configuration::Configuration>,
}

impl TagApiClient {
    pub fn new(configuration: Arc<configuration::Configuration>) -> Self {
        Self { configuration }
    }
}

#[async_trait]
impl TagApi for TagApiClient {
    async fn tag_get(&self, tag: String) -> Result<Vec<String>, Error<TagGetError>>{
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/search", local_var_configuration.base_path);
        let mut local_var_req_builder =
            local_var_client.request(reqwest::Method::GET, local_var_uri_str.as_str());

        let mut query_pairs: Vec<(&str, String)> = Vec::new();
        query_pairs.push(("tag", tag));

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
            let local_var_entity: Option<TagGetError> =
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
pub enum TagGetError {
    UnknownValue(serde_json::Value),
}
