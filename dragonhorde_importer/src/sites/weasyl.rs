use crate::sites::site::{DragonHordeImporterSite, DragonHordeImporterSiteFactory};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use htmd::HtmlToMarkdown;

#[derive(Clone, Debug, Deserialize)]
struct WeasylAvatar {
    url: String,
    mediaid: i64,
}

#[derive(Clone, Debug, Deserialize)]
struct WeasylOwnerMedia {
    avatar: Vec<WeasylAvatar>,
}

#[derive(Clone, Debug, Deserialize)]
struct WeasylMedia {}
impl Default for WeasylMedia {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Weasyl {
    comments: i64,
    description: String,
    embedlink: Option<String>,
    favorited: bool,
    favorites: i64,
    folder_name: Option<String>,
    folder_id: Option<i64>,
    friends_only: bool,
    owner: String,
    owner_login: String,
    owner_media: WeasylOwnerMedia,
    posted_at: DateTime<Utc>,
    rating: String,
    #[serde(skip)]
    media: WeasylMedia,
    submitid: i64,
    subtype: String,
    tags: Vec<String>,
    title: String,
    r#type: String,
    views: i64,
}

impl Weasyl {
    pub async fn new(id: i64, key: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Weasyl-API-Key", key.parse().unwrap());
        let client = reqwest::Client::builder()
            .user_agent("DragonHordeImporter/0.1 (By sl1")
            .default_headers(headers)
            .build()
            .expect("build client");
        let resp = client
            .get(format!(
                "https://www.weasyl.com/api/submissions/{}/view",
                id
            ))
            .send()
            .await?;
        Ok(serde_json::from_str(&resp.text().await?)?)
    }
}

impl DragonHordeImporterSite for Weasyl {
    fn get_tags(
        &self,
        tag_replacements: Option<&HashMap<String, Option<String>>>,
    ) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
        let mut tags: HashMap<String, Vec<String>> = HashMap::new();
        tags.insert("general".to_string(), Vec::new());
        for tag in &self.tags {
            if let Some(tag_replacements) = tag_replacements {
                if let Some(replacement_option) = tag_replacements.get(tag) {
                    // See if we have a replacement value for this tag. If we have an entry but it is None discard the tag
                    if let Some(replacement) = replacement_option {
                        tags.get_mut("general").unwrap().push(replacement.clone());
                    }
                } else {
                    // No Entry, copy the tag
                    tags.get_mut("general").unwrap().push(tag.clone());
                }
            } else {
                tags.get_mut("general").unwrap().push(tag.clone());
            }
        }
        Ok(tags)
    }

    fn get_description(&self) -> Result<String, Box<dyn Error>> {
        let converter = HtmlToMarkdown::builder()
            .skip_tags(vec!["script", "style"])
            .build();
        Ok(converter.convert(&self.description).unwrap())
    }

    fn get_sources(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(vec![format!(
            "https://www.weasyl.com/~{}/submissions/{}/",
            &self.owner_login, self.submitid
        )])
    }

    fn get_image(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        todo!()
    }

    fn get_artists(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(vec![self.owner.clone()])
    }

    fn get_title(&self) -> Result<Option<String>, Box<dyn Error>> {
        Ok(Some(self.title.clone()))
    }

    fn get_created(&self) -> Result<Option<DateTime<Utc>>, Box<dyn Error>> {
        Ok(Some(self.posted_at))
    }
}

#[derive(Debug, Clone)]
pub struct WeasylFactory {
    api_key: String,
}

#[async_trait]
impl DragonHordeImporterSiteFactory for WeasylFactory {
    async fn create(
        &self,
        id: i64,
    ) -> Result<Box<dyn DragonHordeImporterSite + Send>, Box<dyn std::error::Error>> {
        Ok(Box::new(Weasyl::new(id, self.api_key.clone()).await?))
    }
}

impl WeasylFactory {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}
