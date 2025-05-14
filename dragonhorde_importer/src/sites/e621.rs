use crate::sites::site::{DragonHordeImporterSite, DragonHordeImporterSiteFactory};
use crate::sites::weasyl::Weasyl;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use htmd::HtmlToMarkdown;

#[derive(Clone, Debug, Deserialize)]
struct E621File {
    width: i64,
    height: i64,
    ext: String,
    size: i64,
    md5: String,
    url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct E621Preview {
    width: i64,
    height: i64,
    url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct E621Sample {
    has: bool,
    width: i64,
    height: i64,
    url: Option<String>,
    // alternatives: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct E621Score {
    up: i64,
    down: i64,
    total: i64,
}

#[derive(Clone, Debug, Deserialize)]
struct E621Tags {
    general: Vec<String>,
    artist: Vec<String>,
    copyright: Vec<String>,
    character: Vec<String>,
    species: Vec<String>,
    invalid: Vec<String>,
    meta: Vec<String>,
    lore: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct E621Flags {
    pending: bool,
    flagged: bool,
    note_locked: bool,
    status_locked: bool,
    rating_locked: bool,
    deleted: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct E621Relationship {
    parent_id: Option<i64>,
    has_children: bool,
    has_active_children: bool,
    children: Vec<i64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct e621 {
    id: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    file: E621File,
    preview: E621Preview,
    sample: E621Sample,
    score: E621Score,
    tags: E621Tags,
    locked_tags: Option<Vec<String>>,
    change_seq: i64,
    flags: E621Flags,
    rating: String,
    fav_count: i64,
    sources: Vec<String>,
    pools: Vec<i64>,
    relationships: E621Relationship,
    approver_id: Option<i64>,
    uploader_id: i64,
    description: String,
    comment_count: i64,
    is_favorited: bool,
    has_notes: bool,
    duration: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
struct E621Result {
    post: e621,
}

impl e621 {
    pub async fn new(id: i64, client: Client) -> Result<Self, Box<dyn std::error::Error>> {
        let resp = client
            .get(format!("https://e621.net/posts/{}.json", id))
            .send()
            .await?;
        let string = &resp.text().await?;
        // dbg!(&string);
        let result: E621Result = serde_json::from_str(&string)?;
        Ok(result.post)
    }
}

enum E621TagCategories {
    General,
    Artist,
    Copyright,
    Character,
    Species,
    Invalid,
    Meta,
    Lore,
}

impl E621TagCategories {
    fn as_str(&self) -> &'static str {
        match self {
            E621TagCategories::General => "general",
            E621TagCategories::Artist => "artist",
            E621TagCategories::Copyright => "copyright",
            E621TagCategories::Character => "character",
            E621TagCategories::Species => "species",
            E621TagCategories::Invalid => "invalid",
            E621TagCategories::Meta => "meta",
            E621TagCategories::Lore => "lore",
        }
    }
}

impl DragonHordeImporterSite for e621 {
    fn get_tags(
        &self,
        tag_replacements: Option<&HashMap<String, Option<String>>>,
    ) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
        let mut tags: HashMap<String, Vec<String>> = HashMap::new();
        for category in [
            E621TagCategories::General,
            E621TagCategories::Copyright,
            E621TagCategories::Character,
            E621TagCategories::Species,
            E621TagCategories::Invalid,
            E621TagCategories::Meta,
            E621TagCategories::Lore,
        ] {
            tags.insert(category.as_str().parse().unwrap(), Vec::new());
            if let Some(tag_replacements) = tag_replacements {
                for tag in match category {
                    E621TagCategories::General => &self.tags.general,
                    E621TagCategories::Copyright => &self.tags.copyright,
                    E621TagCategories::Character => &self.tags.character,
                    E621TagCategories::Species => &self.tags.species,
                    E621TagCategories::Invalid => &self.tags.invalid,
                    E621TagCategories::Meta => &self.tags.meta,
                    E621TagCategories::Lore => &self.tags.lore,
                    E621TagCategories::Artist => todo!(),
                } {
                    if let Some(replacement_option) = tag_replacements.get(tag) {
                        // See if we have a replacement value for this tag. If we have an entry but it is None discard the tag
                        if let Some(replacement) = replacement_option {
                            tags.get_mut(category.as_str())
                                .unwrap()
                                .push(replacement.clone());
                        }
                    } else {
                        // No Entry, copy the tag
                        tags.get_mut(category.as_str()).unwrap().push(tag.clone());
                    }
                }
            } else {
                // No replacements provided, just copy the tags
                for category in [
                    E621TagCategories::General,
                    E621TagCategories::Copyright,
                    E621TagCategories::Character,
                    E621TagCategories::Species,
                    E621TagCategories::Invalid,
                    E621TagCategories::Meta,
                    E621TagCategories::Lore,
                ] {
                    tags.insert(category.as_str().parse().unwrap(), Vec::new());
                    for tag in match category {
                        E621TagCategories::General => &self.tags.general,
                        E621TagCategories::Copyright => &self.tags.copyright,
                        E621TagCategories::Character => &self.tags.character,
                        E621TagCategories::Species => &self.tags.species,
                        E621TagCategories::Invalid => &self.tags.invalid,
                        E621TagCategories::Meta => &self.tags.meta,
                        E621TagCategories::Lore => &self.tags.lore,
                        E621TagCategories::Artist => todo!(),
                    } {
                        tags.get_mut(category.as_str()).unwrap().push(tag.clone());
                    }
                }
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
        let mut sources = self.sources.clone()
            .into_iter()
            .filter(|s| !s.starts_with("https://d.furaffinity.net/"))
            .filter(|s| !s.starts_with("https://pbs.twimg.com/"))
            .collect::<Vec<String>>();
        sources.push(format!("https://e621.net/posts/{}", self.id));
        Ok(sources)
    }

    fn get_image(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        todo!()
    }

    fn get_artists(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(self
            .tags
            .artist
            .clone()
            .into_iter()
            .filter(|a| !a.eq("conditional_dnp"))
            .collect())
    }

    fn get_title(&self) -> Result<Option<String>, Box<dyn Error>> {
        Ok(None)
    }

    fn get_created(&self) -> Result<Option<DateTime<Utc>>, Box<dyn Error>> {
        Ok(Some(self.created_at))
    }
}

#[derive(Debug, Clone)]
pub struct E621Factory {
    client: Client,
}

#[async_trait]
impl DragonHordeImporterSiteFactory for E621Factory {
    async fn create(
        &self,
        id: i64,
    ) -> Result<Box<dyn DragonHordeImporterSite + Send>, Box<dyn std::error::Error>> {
        Ok(Box::new(e621::new(id, self.client.clone()).await?))
    }
}

impl E621Factory {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}
