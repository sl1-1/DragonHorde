/*
 * DragonHorde
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: 1.0
 *
 * Generated by: https://openapi-generator.tech
 */
use std::path::Path;
use async_trait::async_trait;
use reqwest;
use std::sync::Arc;
use reqwest::multipart;
use serde::{Deserialize, Serialize, de::Error as _};
use crate::{models};
use super::{Error, configuration, urlencode, ResponseContent, ContentType};

#[async_trait]
pub trait MediaApi: Send + Sync {

    /// GET /media
    ///
    ///
    async fn media_get(&self, last: Option<i64>, per_page: Option<i64>) -> Result<models::media::SearchResult, Error<MediaGetError>>;

    /// DELETE /media/{id}
    ///
    ///
    async fn media_id_delete(&self, id: i64) -> Result<(), Error<MediaIdDeleteError>>;

    /// GET /media/{id}/file
    ///
    ///
    async fn media_id_file_get(&self, id: i64) -> Result<Vec<u8>, Error<MediaIdFileGetError>>;

    /// GET /media/{id}
    ///
    ///
    async fn media_id_get(&self, id: i64) -> Result<models::Media, Error<MediaIdGetError>>;

    /// PUT /media/{id}
    ///
    ///
    async fn media_id_put(&self, id: i64, media: models::Media) -> Result<models::Media, Error<MediaIdPutError>>;

    /// PUT /media/{id}
    ///
    ///
    async fn media_id_patch(&self, id: i64, media: models::Media) -> Result<(), Error<MediaIdPutError>>;

    /// GET /media/{id}/thumbnail
    ///
    ///
    async fn media_id_thumbnail_get(&self, id: i64) -> Result<Vec<u8>, Error<MediaIdThumbnailGetError>>;

    /// POST /media
    ///
    ///
    async fn media_post<'media>(&self, media: models::Media, file: &Path) -> Result<models::Media, Error<MediaPostError>>;
}

pub struct MediaApiClient {
    configuration: Arc<configuration::Configuration>
}

impl MediaApiClient {
    pub fn new(configuration: Arc<configuration::Configuration>) -> Self {
        Self { configuration }
    }
}



#[async_trait]
impl MediaApi for MediaApiClient {
    async fn media_get(&self, last: Option<i64>, per_page: Option<i64>) -> Result<models::media::SearchResult, Error<MediaGetError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media", local_var_configuration.base_path);
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::GET, local_var_uri_str.as_str());

        let mut query_pairs:Vec<(&str, String)> = Vec::new();
        if last.is_some(){
            query_pairs.push(("last", last.unwrap().to_string()));
        }
        if per_page.is_some(){
            query_pairs.push(("per_page", per_page.unwrap().to_string()));
        }

        local_var_req_builder = local_var_req_builder.query(&query_pairs);
        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
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
                ContentType::Text => return Err(Error::from(serde_json::Error::custom("Received `text/plain` content type response that cannot be converted to `Vec&lt;models::Media&gt;`"))),
                ContentType::Binary => Err(Error::from(serde_json::Error::custom("Received `application/octet` content type response that cannot be converted to `Vec&lt;models::Media&gt;`"))),
                ContentType::Unsupported(local_var_unknown_type) => return Err(Error::from(serde_json::Error::custom(format!("Received `{local_var_unknown_type}` content type response that cannot be converted to `Vec&lt;models::Media&gt;`")))),
            }
        } else {
            let local_var_entity: Option<MediaGetError> = serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent { status: local_var_status, content: local_var_content, entity: local_var_entity };
            Err(Error::ResponseError(local_var_error))
        }
    }
    async fn media_id_delete(&self, id: i64) -> Result<(), Error<MediaIdDeleteError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media/{id}", local_var_configuration.base_path, id=urlencode(id.to_string()));
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::DELETE, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }

        let local_var_req = local_var_req_builder.build()?;
        let local_var_resp = local_var_client.execute(local_var_req).await?;

        let local_var_status = local_var_resp.status();
        let local_var_content = local_var_resp.text().await?;

        if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
            Ok(())
        } else {
            let local_var_entity: Option<MediaIdDeleteError> = serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent { status: local_var_status, content: local_var_content, entity: local_var_entity };
            Err(Error::ResponseError(local_var_error))
        }
    }

    async fn media_id_file_get(&self, id: i64) -> Result<Vec<u8>, Error<MediaIdFileGetError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media/{id}/file", local_var_configuration.base_path, id=urlencode(id.to_string()));
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::GET, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }

        let local_var_req = local_var_req_builder.build()?;
        let local_var_resp = local_var_client.execute(local_var_req).await?;

        let local_var_status = local_var_resp.status();

        if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
            Ok(Vec::from(local_var_resp.bytes().await?))
        } else {
            let local_var_error = ResponseContent { status: local_var_status, content: "".to_string(), entity: None };
            Err(Error::ResponseError(local_var_error))
        }
    }

    async fn media_id_get(&self, id: i64) -> Result<models::Media, Error<MediaIdGetError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media/{id}", local_var_configuration.base_path, id=urlencode(id.to_string()));
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::GET, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
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
                ContentType::Text => Err(Error::from(serde_json::Error::custom("Received `text/plain` content type response that cannot be converted to `models::Media`"))),
                ContentType::Binary => Err(Error::from(serde_json::Error::custom("Received `application/octet` content type response that cannot be converted to `Vec&lt;String&gt;`"))),
                ContentType::Unsupported(local_var_unknown_type) => Err(Error::from(serde_json::Error::custom(format!("Received `{local_var_unknown_type}` content type response that cannot be converted to `models::Media`")))),
            }
        } else {
            let local_var_entity: Option<MediaIdGetError> = serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent { status: local_var_status, content: local_var_content, entity: local_var_entity };
            Err(Error::ResponseError(local_var_error))
        }
    }

    async fn media_id_put(&self, id: i64, media: models::Media) -> Result<models::Media, Error<MediaIdPutError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media/{id}", local_var_configuration.base_path, id=urlencode(id.to_string()));
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::PUT, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }
        local_var_req_builder = local_var_req_builder.json(&media);

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
                ContentType::Text => Err(Error::from(serde_json::Error::custom("Received `text/plain` content type response that cannot be converted to `models::Media`"))),
                ContentType::Binary => Err(Error::from(serde_json::Error::custom("Received `application/octet` content type response that cannot be converted to `Vec&lt;String&gt;`"))),
                ContentType::Unsupported(local_var_unknown_type) => Err(Error::from(serde_json::Error::custom(format!("Received `{local_var_unknown_type}` content type response that cannot be converted to `models::Media`")))),
            }
        } else {
            let local_var_entity: Option<MediaIdPutError> = serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent { status: local_var_status, content: local_var_content, entity: local_var_entity };
            Err(Error::ResponseError(local_var_error))
        }
    }

    async fn media_id_patch(&self, id: i64, media: models::Media) -> Result<(), Error<MediaIdPutError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media/{id}", local_var_configuration.base_path, id=urlencode(id.to_string()));
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::PATCH, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }
        local_var_req_builder = local_var_req_builder.json(&media);

        let local_var_req = local_var_req_builder.build()?;
        let local_var_resp = local_var_client.execute(local_var_req).await?;

        let local_var_status = local_var_resp.status();
        let local_var_content = local_var_resp.text().await?;

        if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
            Ok(())
        } else {
            let local_var_entity: Option<MediaIdPutError> = serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent { status: local_var_status, content: local_var_content, entity: local_var_entity };
            Err(Error::ResponseError(local_var_error))
        }
    }

    async fn media_id_thumbnail_get(&self, id: i64) -> Result<Vec<u8>, Error<MediaIdThumbnailGetError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media/{id}/thumbnail", local_var_configuration.base_path, id=urlencode(id.to_string()));
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::GET, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }

        let local_var_req = local_var_req_builder.build()?;
        let local_var_resp = local_var_client.execute(local_var_req).await?;

        let local_var_status = local_var_resp.status();

        if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
            Ok(Vec::from(local_var_resp.bytes().await?))
        } else {
            let local_var_error = ResponseContent { status: local_var_status, content: "".to_string(), entity: None };
            Err(Error::ResponseError(local_var_error))
        }
    }

    async fn media_post<'media>(&self, media: models::Media, file: &Path) -> Result<models::Media, Error<MediaPostError>> {
        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!("{}/media", local_var_configuration.base_path);
        let mut local_var_req_builder = local_var_client.request(reqwest::Method::POST, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder.header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }

        let form = multipart::Form::new()
            .text("data", serde_json::to_string(&media).expect("Serializing media failed"))
            .file("file", file)
            .await?;
        
        local_var_req_builder = local_var_req_builder.multipart(form);
        
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
                ContentType::Text => Err(Error::from(serde_json::Error::custom("Received `text/plain` content type response that cannot be converted to `models::Media`"))),
                ContentType::Binary => Err(Error::from(serde_json::Error::custom("Received `application/octet` content type response that cannot be converted to `models::Media`"))),

                ContentType::Unsupported(local_var_unknown_type) => Err(Error::from(serde_json::Error::custom(format!("Received `{local_var_unknown_type}` content type response that cannot be converted to `models::Media`")))),
            }
        } else {
            let local_var_entity: Option<MediaPostError> = serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent { status: local_var_status, content: local_var_content, entity: local_var_entity };
            Err(Error::ResponseError(local_var_error))
        }
    }

}

/// struct for typed errors of method [`media_get`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaGetError {
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_delete`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdDeleteError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_file_get`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdFileGetError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_get`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdGetError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_put`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdPutError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_sources_delete`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdGenericDeleteError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_sources_get`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdGenericGetError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_tags_delete`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdTagsDeleteError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_tags_get`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdTagsGetError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_tags_put`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdGenericPutError {
    Status404(),
    Status409(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_id_thumbnail_get`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaIdThumbnailGetError {
    Status404(),
    Status500(),
    UnknownValue(serde_json::Value),
}

/// struct for typed errors of method [`media_post`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaPostError {
    UnknownValue(serde_json::Value),
}

