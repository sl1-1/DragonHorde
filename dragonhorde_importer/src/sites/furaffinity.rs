use crate::sites::error;
use crate::sites::site::{DragonHordeImporterSite, DragonHordeImporterSiteFactory};
use async_trait::async_trait;
use furaffinity_rs::{FurAffinity, Submission};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use chrono::{DateTime, Utc};
use htmd::HtmlToMarkdown;

#[derive(Debug, Clone)]
pub struct Furaffinity {
    post: furaffinity_rs::Submission,
}
impl Furaffinity {
    pub async fn new(
        id: i64,
        cookie_a: String,
        cookie_b: String,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let fa = FurAffinity::new(
            cookie_a,
            cookie_b,
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:137.0) Gecko/20100101 Firefox/137.0"
                .to_string(),
            None,
        )
        .get_submission(id as i32)
        .await?;
        match fa {
            None => Ok(None),
            Some(fa) => Ok(Some(Self { post: fa })),
        }
    }
}

impl DragonHordeImporterSite for Furaffinity {
    fn get_tags(
        &self,
        tag_replacements: Option<&HashMap<String, Option<String>>>,
    ) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
        let mut tags: HashMap<String, Vec<String>> = HashMap::new();
        tags.insert("general".to_string(), Vec::new());
        for tag in &self.post.tags {
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
        if self.post.species != "Unspecified / Any" {
            tags.insert("species".to_string(), Vec::new());
            if let Some(tag_replacements) = tag_replacements {
                if let Some(replacement_option) = tag_replacements.get(&self.post.species) {
                    if let Some(replacement) = replacement_option {
                        tags.get_mut("species").unwrap().push(replacement.clone());
                    }
                } else {
                    // No Entry, copy the tag
                    tags.get_mut("species")
                        .unwrap()
                        .push(self.post.species.clone());
                }
            } else {
                tags.get_mut("species")
                    .unwrap()
                    .push(self.post.species.clone());
            }
        }
        if !["All", "Multiple characters", "Other / Not Specified"]
            .iter()
            .any(|x| self.post.gender == *x)
        {
            tags.insert("gender".to_string(), Vec::new());
            if let Some(tag_replacements) = tag_replacements {
                if let Some(replacement_option) = tag_replacements.get(&self.post.gender) {
                    if let Some(replacement) = replacement_option {
                        tags.get_mut("gender").unwrap().push(replacement.clone());
                    }
                } else {
                    // No Entry, copy the tag
                    tags.get_mut("gender")
                        .unwrap()
                        .push(self.post.gender.clone());
                }
            } else {
                tags.get_mut("gender")
                    .unwrap()
                    .push(self.post.gender.clone());
            }
        }
        if !["All", "Miscellaneous", "General Furry Art"]
            .iter()
            .any(|x| self.post.category == *x)
        {
            if let Some(tag_replacements) = tag_replacements {
                if let Some(replacement_option) = tag_replacements.get(&self.post.category) {
                    if let Some(replacement) = replacement_option {
                        tags.get_mut("general").unwrap().push(replacement.clone());
                    }
                } else {
                    // No Entry, copy the tag
                    tags.get_mut("general")
                        .unwrap()
                        .push(self.post.category.clone());
                }
            } else {
                tags.get_mut("general")
                    .unwrap()
                    .push(self.post.category.clone());
            }
        }

        Ok(tags)
    }

    fn get_description(&self) -> Result<String, Box<dyn Error>> {
        let converter = HtmlToMarkdown::builder()
            .skip_tags(vec!["script", "style"])
            .build();
        Ok(converter.convert(&self.post.description).unwrap())
    }

    fn get_sources(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(vec![format!(
            "https://www.furaffinity.net/view/{}/",
            self.post.id
        )])
    }

    fn get_image(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        todo!()
    }

    fn get_artists(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(vec![self.post.artist.clone()])
    }

    fn get_title(&self) -> Result<Option<String>, Box<dyn Error>> {
        Ok(Some(self.post.title.clone()))
    }

    fn get_created(&self) -> Result<Option<DateTime<Utc>>, Box<dyn Error>> {
        Ok(Some(self.post.posted_at))
    }
}

#[derive(Debug, Clone)]
pub struct FuraffinityFactory {
    cookie_a: String,
    cookie_b: String,
}

#[async_trait]
impl DragonHordeImporterSiteFactory for FuraffinityFactory {
    async fn create(
        &self,
        id: i64,
    ) -> Result<Box<dyn DragonHordeImporterSite + Send>, Box<dyn std::error::Error>> {
        match Furaffinity::new(id, self.cookie_a.clone(), self.cookie_b.clone()).await {
            Ok(site) => match site {
                None => Err(Box::new(error::NotFound)),
                Some(fa) => Ok(Box::new(fa)),
            },
            Err(e) => Err(e),
        }
    }
}

impl FuraffinityFactory {
    pub fn new(cookie_a: String, cookie_b: String) -> Self {
        Self { cookie_a, cookie_b }
    }
}
