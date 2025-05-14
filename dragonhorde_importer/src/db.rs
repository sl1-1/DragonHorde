use crate::models::fuzzysearch::{ActiveModel, Entity};
use config::Config;
use log::{error, info, warn};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema, Set,
    TransactionTrait,
};
use std::path::PathBuf;
use sea_orm::prelude::DateTimeWithTimeZone;

const RECORDS_AT_ONCE: usize = 2000;
const RECORDS_PER_TRANSACTION: usize = 2000;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CsvModel<'a> {
    pub(crate) site: &'a str,
    pub id: i64,
    pub artists: Option<&'a str>,
    pub(crate) hash: i64,
    posted_at: Option<DateTimeWithTimeZone>,
    updated_at: Option<DateTimeWithTimeZone>,
    pub(crate) sha256: Option<&'a str>,
    deleted: bool,
    content_url: Option<&'a str>,
}

pub async fn create_db(settings: &Config, force: &bool, in_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path: PathBuf = settings.get("home")?;
    db_path.push(settings.get_string("db")?);

    if *force {
        std::fs::remove_file(&db_path)?;
    }

    let db: DatabaseConnection =
        Database::connect(format!("sqlite://{}?mode=rwc", db_path.to_str().unwrap())).await?;

    let builder = DbBackend::Sqlite;
    let schema = Schema::new(builder);

    db.execute(builder.build(&schema.create_table_from_entity(Entity))).await?;

    for statement in schema.create_index_from_entity(Entity) {
        db.execute(builder.build(&statement)).await?;
    }
    let fuzzy_csv = match std::fs::File::open(in_file) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open csv file: {}", e);
            panic!("Failed to open csv file");
        }
    };
    let mut rdr = csv::Reader::from_reader(fuzzy_csv);
    let mut records: Vec<ActiveModel> = Vec::new();
    let mut raw_record = csv::StringRecord::new();
    let headers = rdr.headers().unwrap().clone();
    let mut i = 0;
    let mut txn = db.begin().await?;
    while rdr.read_record(&mut raw_record).unwrap() {
        match raw_record.deserialize::<CsvModel>(Some(&headers)) {
            Ok(r) => {
                records.push(ActiveModel {
                    key: Default::default(),
                    site: Set(r.site.to_string()),
                    id: Set(r.id),
                    artists: if r.artists.is_none() {
                        Default::default()
                    } else {
                        Set(Some(r.artists.unwrap().to_string()))
                    },
                    hash: Set(r.hash.clone()),
                    posted_at: if r.posted_at.is_none() {
                        Default::default()
                    } else {
                        Set(r.posted_at)
                    },
                    updated_at: if r.updated_at.is_none() {
                        Default::default()
                    } else {
                        Set(r.updated_at)
                    },
                    sha256: if r.sha256.is_none() {
                        Default::default()
                    } else {
                        Set(Some(r.sha256.unwrap().to_string()))
                    },
                    deleted: Set(r.deleted),
                    content_url: if r.content_url.is_none() {
                        Default::default()
                    } else {
                        Set(Some(r.content_url.unwrap().to_string()))
                    },
                });
                if records.len() >= RECORDS_AT_ONCE {
                    Entity::insert_many(records).exec(&txn).await?;
                    records = Vec::new();
                    i = i + 1;
                    if i >= RECORDS_PER_TRANSACTION {
                        txn.commit().await?;
                        txn = db.begin().await?;
                        info!("Inserted {} records", i * RECORDS_AT_ONCE);
                        i = 0;
                    }
                }
            }
            Err(_) => {}
        };
    }
    Entity::insert_many(records).exec(&txn).await?;
    txn.commit().await?;
    Ok(())
}

pub async fn db_init(settings: &Config) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let mut db_path: PathBuf = settings.get("home")?;
    db_path.push(settings.get_string("db")?);

    let db: DatabaseConnection =
        Database::connect(format!("sqlite://{}?mode=ro", db_path.to_str().unwrap())).await?;
    Ok(db)
}
