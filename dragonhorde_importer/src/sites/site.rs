use async_trait::async_trait;
use std::collections::HashMap;

pub trait DragonHordeImporterSite {
    fn get_tags(
        &self,
        tag_replacements: Option<&HashMap<String, Option<String>>>,
    ) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>>;

    fn get_description(&self) -> Result<String, Box<dyn std::error::Error>>;

    fn get_sources(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    fn get_image(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    fn get_artists(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    fn get_title(&self) -> Result<Option<String>, Box<dyn std::error::Error>>;
    
    fn get_created(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>, Box<dyn std::error::Error>>;
}


#[async_trait]
pub trait DragonHordeImporterSiteFactory {
    async fn create(
        &self,
        id: i64,
    ) -> Result<Box<dyn DragonHordeImporterSite + Send>, Box<dyn std::error::Error>>;
}
