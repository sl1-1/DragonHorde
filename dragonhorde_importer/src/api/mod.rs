use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ResponseContent<T> {
    pub status: reqwest::StatusCode,
    pub content: String,
    pub entity: Option<T>,
}

#[derive(Debug)]
pub enum Error<T> {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Io(std::io::Error),
    ResponseError(ResponseContent<T>),
}

impl <T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (module, e) = match self {
            Error::Reqwest(e) => ("reqwest", e.to_string()),
            Error::Serde(e) => ("serde", e.to_string()),
            Error::Io(e) => ("IO", e.to_string()),
            Error::ResponseError(e) => ("response", format!("status code {}", e.status)),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl <T: fmt::Debug> error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Error::Reqwest(e) => e,
            Error::Serde(e) => e,
            Error::Io(e) => e,
            Error::ResponseError(_) => return None,
        })
    }
}

impl <T> From<reqwest::Error> for Error<T> {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl <T> From<serde_json::Error> for Error<T> {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e)
    }
}

impl <T> From<std::io::Error> for Error<T> {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub fn urlencode<T: AsRef<str>>(s: T) -> String {
    ::url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}

pub fn parse_deep_object(prefix: &str, value: &serde_json::Value) -> Vec<(String, String)> {
    if let serde_json::Value::Object(object) = value {
        let mut params = vec![];

        for (key, value) in object {
            match value {
                serde_json::Value::Object(_) => params.append(&mut parse_deep_object(
                    &format!("{}[{}]", prefix, key),
                    value,
                )),
                serde_json::Value::Array(array) => {
                    for (i, value) in array.iter().enumerate() {
                        params.append(&mut parse_deep_object(
                            &format!("{}[{}][{}]", prefix, key, i),
                            value,
                        ));
                    }
                },
                serde_json::Value::String(s) => params.push((format!("{}[{}]", prefix, key), s.clone())),
                _ => params.push((format!("{}[{}]", prefix, key), value.to_string())),
            }
        }

        return params;
    }

    unimplemented!("Only objects are supported with style=deepObject")
}

/// Internal use only
/// A content type supported by this client.
#[allow(dead_code)]
enum ContentType {
    Json,
    Text,
    Binary,
    Unsupported(String)
}

impl From<&str> for ContentType {
    fn from(content_type: &str) -> Self {
        if content_type.starts_with("application") && content_type.contains("json") {
            return Self::Json;
        } else if content_type.starts_with("text/plain") {
            return Self::Text;
        } else if content_type.starts_with("application/octet-stream") {
            return Self::Binary;
        } else {
            return Self::Unsupported(content_type.to_string());
        }
    }
}

// pub mod collection_api;
// pub mod creator_api;
pub mod media_api;
// pub mod search_api;
// pub mod tag_group_api;

pub mod configuration;

use std::sync::Arc;

pub trait Api {
    // fn collection_api(&self) -> &dyn collection_api::CollectionApi;
    // fn creator_api(&self) -> &dyn creator_api::CreatorApi;
    fn media_api(&self) -> &dyn media_api::MediaApi;
    // fn search_api(&self) -> &dyn search_api::SearchApi;
    // fn tag_group_api(&self) -> &dyn tag_group_api::TagGroupApi;
}

pub struct ApiClient {
    // collection_api: Box<dyn collection_api::CollectionApi>,
    // creator_api: Box<dyn creator_api::CreatorApi>,
    media_api: Box<dyn media_api::MediaApi>,
    // search_api: Box<dyn search_api::SearchApi>,
    // tag_group_api: Box<dyn tag_group_api::TagGroupApi>,
}

impl ApiClient {
    pub fn new(configuration: Arc<configuration::Configuration>) -> Self {
        Self {
            // collection_api: Box::new(collection_api::CollectionApiClient::new(configuration.clone())),
            // creator_api: Box::new(creator_api::CreatorApiClient::new(configuration.clone())),
            media_api: Box::new(media_api::MediaApiClient::new(configuration.clone())),
            // search_api: Box::new(search_api::SearchApiClient::new(configuration.clone())),
            // tag_group_api: Box::new(tag_group_api::TagGroupApiClient::new(configuration.clone())),
        }
    }
}

impl Api for ApiClient {
    // fn collection_api(&self) -> &dyn collection_api::CollectionApi {
    //     self.collection_api.as_ref()
    // }
    // fn creator_api(&self) -> &dyn creator_api::CreatorApi {
    //     self.creator_api.as_ref()
    // }
    fn media_api(&self) -> &dyn media_api::MediaApi {
        self.media_api.as_ref()
    }
    // fn search_api(&self) -> &dyn search_api::SearchApi {
    //     self.search_api.as_ref()
    // }
    // fn tag_group_api(&self) -> &dyn tag_group_api::TagGroupApi {
    //     self.tag_group_api.as_ref()
    // }
}


