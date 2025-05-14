use crate::db::{create_db, db_init};
use crate::models::fuzzysearch::{Column, Entity, Model};
use crate::sites::e621::E621Factory;
use crate::sites::furaffinity::FuraffinityFactory;
use crate::sites::site::{DragonHordeImporterSite, DragonHordeImporterSiteFactory};
use crate::sites::weasyl::WeasylFactory;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use config::{Config, File};
use dragonhorde_api_client::api::configuration::Configuration;
use dragonhorde_api_client::api::{Api, ApiClient};
use dragonhorde_api_client::models::Media;
use img_hash::HashAlg::Gradient;
use img_hash::HasherConfig;
use log::{LevelFilter, debug, error, info, warn};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{PathBuf};
use std::sync::{Arc, Mutex};

mod db;
mod models;
mod sites;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum SiteId {
    Furaffinity,
    Weasyl,
    E621,
}

struct SiteFactories {
    furaffinity_factory: Option<FuraffinityFactory>,
    weasyl_factory: Option<WeasylFactory>,
    e621_factory: Option<E621Factory>,
}

fn phash(path: &PathBuf) -> Result<i64, Box<dyn std::error::Error>> {
    let reader = image::io::Reader::open(path)?;
    let im = reader.decode()?;

    let image_hash = HasherConfig::with_bytes_type::<[u8; 8]>()
        .hash_alg(Gradient)
        .hash_size(8, 8)
        .preproc_dct()
        .to_hasher()
        .hash_image(&im);
    let hash: [u8; 8] = image_hash.as_bytes().try_into()?;
    Ok(i64::from_be_bytes(hash))
}

fn sha256_hash(path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let file: Vec<u8> = fs::read(path).expect("Hash File");
    let mut hasher = Sha256::new();
    hasher.update(&file);
    let hash = hasher.finalize();
    Ok(base64::encode(hash))
}

async fn get_post(
    sites: &SiteFactories,
    site: &SiteId,
    id: i64,
) -> Option<Box<dyn DragonHordeImporterSite + Send>> {
    match site {
        SiteId::Furaffinity => match sites.furaffinity_factory.clone() {
            None => None,
            Some(site) => match site.create(id).await {
                Ok(entry) => Some(entry),
                Err(_) => None,
            },
        },
        SiteId::Weasyl => match sites.weasyl_factory.clone() {
            None => None,
            Some(site) => match site.create(id).await {
                Ok(entry) => Some(entry),
                Err(_) => None,
            },
        },
        SiteId::E621 => match sites.e621_factory.clone() {
            None => None,
            Some(site) => match site.create(id).await {
                Ok(entry) => Some(entry),
                Err(_) => None,
            },
        },
    }
}

async fn make_media2(
    matched: Vec<Model>,
    sites: &SiteFactories,
) -> Result<Media, Box<dyn std::error::Error>> {
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    let mut sources: Vec<String> = Vec::new();
    let mut artists: Vec<String> = Vec::new();
    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut created: Option<DateTime<Utc>> = None;
    for entry in matched {
        let site = match entry.site.as_str() {
            "furaffinity" => SiteId::Furaffinity,
            "weasyl" => SiteId::Weasyl,
            "e621" => SiteId::E621,
            _ => {
                error!("Unsupported Site {}", entry.site.as_str());
                continue;
            }
        };

        let mut post: Option<Box<dyn DragonHordeImporterSite + Send>> = None;

        //No point trying to load the post if we know it's deleted!
        if !entry.deleted {
            post = get_post(sites, &site, entry.id).await
        }

        if let Some(post) = post {
            // Use the oldest created at date.
            if created.is_none() {
                created = post.get_created()?;
            }

            //Use the first title we get
            if title.is_none() {
                title = post.get_title()?
            }

            let local_tags = post.get_tags(None)?;
            for group_string in local_tags.keys() {
                if let Some(group) = tags.get_mut(group_string) {
                    group.extend(local_tags.get(group_string).unwrap().clone());
                } else {
                    tags.insert(
                        group_string.to_string(),
                        local_tags.get(group_string).unwrap().clone(),
                    );
                }
            }

            match post.get_description() {
                Ok(d) => {
                    if !d.is_empty() {
                        let mut _description = d;
                        _description = format!(
                            "From: {}@{:?}\n{}\n\n",
                            post.get_artists()?.get(0).unwrap_or(&"".to_string()),
                            site,
                            _description
                        );
                        if description.is_none() {
                            description = Some(_description)
                        } else {
                            description =
                                Some(format!("{}\n\n{}", description.unwrap(), _description));
                        }
                    }
                }
                Err(e) => return Err(e),
            }

            sources.extend(post.get_sources()?);
            artists.extend(post.get_artists()?);
        } else {
            // Use values from the fuzzysearch database if the information isn't available online
            if created.is_none() {
                if let Some(posted) = entry.posted_at {
                    created = Some(posted.to_utc());
                }
            }
            if let Some(artist) = &entry.artists {
                artists.extend(artist.split(',').map(|a| a.to_string()));
            }

            //No point including the source when it no longer can be visited
            if !entry.deleted {
                let source: String = match &site {
                    SiteId::Furaffinity => {
                        format!("https://www.furaffinity.net/view/{}/", entry.id)
                    }
                    SiteId::Weasyl => {
                        format!(
                            "https://www.weasyl.com/~{}/submissions/{}/",
                            entry.artists.unwrap(),
                            entry.id
                        )
                    }
                    SiteId::E621 => {
                        format!("https://e621.net/posts/{}", entry.id)
                    }
                };
                sources.push(source);
            }
        }
    }
    artists.sort_by_key(|a| a.to_lowercase());
    artists.dedup_by_key(|a| a.to_lowercase());
    sources.sort_by_key(|s| s.to_lowercase());
    sources.dedup_by_key(|s| s.to_lowercase());

    for group in tags.iter_mut() {
        group.1.sort_by_key(|t| t.to_lowercase());
        group.1.dedup_by_key(|t| t.to_lowercase());
    }

    Ok(Media {
        id: None,
        storage_uri: None,
        sha256: None,
        perceptual_hash: None,
        uploaded: None,
        created,
        title,
        creators: if artists.is_empty() {
            None
        } else {
            Some(artists)
        },
        tag_groups: if tags.is_empty() { None } else { Some(tags) },
        sources: if sources.is_empty() {
            None
        } else {
            Some(sources)
        },
        collections: None,
        description,
    })
}

async fn run(settings: &Config, in_folder: &PathBuf, out_folder: &Option<PathBuf>) {
    let in_queue: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));

    let paths = fs::read_dir(&in_folder).unwrap();
    in_queue
        .lock()
        .unwrap()
        .extend(paths.into_iter().map(|p| p.unwrap().path()));

    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    let mut sites = SiteFactories {
        furaffinity_factory: None,
        weasyl_factory: None,
        e621_factory: None,
    };

    if let Ok(fa_settings) = settings.get_table("furaffinity") {
        if fa_settings.contains_key("cookie_a") && fa_settings.contains_key("cookie_b") {
            sites.furaffinity_factory = Some(FuraffinityFactory::new(
                fa_settings.get("cookie_a").unwrap().to_string(),
                fa_settings.get("cookie_b").unwrap().to_string(),
            ));
        }
    }

    if let Ok(weasyl_settings) = settings.get_table("weasyl") {
        if weasyl_settings.contains_key("api_key") {
            sites.weasyl_factory = Some(WeasylFactory::new(
                weasyl_settings.get("api_key").unwrap().to_string(),
            ));
        }
    }

    let client = reqwest::Client::builder()
        .user_agent("DragonHordeImporter/0.1 (By sl1")
        .build()
        .expect("build client");

    sites.e621_factory = Some(E621Factory::new(client));

    let sites: Arc<SiteFactories> = Arc::new(sites);

    fn check_queue(queue: Arc<Mutex<Vec<PathBuf>>>) -> bool {
        queue.lock().unwrap().len() > 0
    }

    for i in 0..10 {
        println!("Spawning Thread {}", i);
        let _queue = Arc::clone(&in_queue);
        // let _db = db.clone();
        let _sites = sites.clone();
        let _settings = settings.clone();
        let api_config = Arc::new(Configuration {
            base_path: settings.get_string("server").unwrap(),
            ..Default::default()
        });
        let client = ApiClient::new(api_config);
        let _out_folder: Option<PathBuf> = if out_folder.is_some() {Some(PathBuf::from(out_folder.clone().unwrap()))} else {None};


        let handle = tokio::spawn(async move {
            let _db = match db_init(&_settings).await {
                Ok(db) => db,
                Err(e) => {
                    error!("{}", e);
                    panic!("{}", e)
                }
            };

            while check_queue(_queue.clone()) {
                let file = _queue.lock().unwrap().pop().unwrap();
                let image_hash = match phash(&file) {
                    Ok(image_hash) => image_hash,
                    Err(e) => {
                        error!("{:?} {:?}", file, e);
                        continue;
                    }
                };

                let mut matches = match Entity::find()
                    .filter(Column::Hash.eq(image_hash))
                    .all(&_db)
                    .await
                {
                    Ok(m) => m,
                    Err(e) => {
                        error!("{:?} {:?}", file, e);
                        continue;
                    }
                };

                if matches.is_empty() {
                    info!("{:?}: No matches found", file.file_name());
                    continue;
                }

                matches.sort_by_key(|m| {
                    if m.posted_at.is_none() {
                        Utc::now()
                    } else {
                        DateTime::from(m.posted_at.unwrap())
                    }
                });

                debug!("{}: {:?}, {:?}", i, &file.file_name(), &matches);

                let model = match make_media2(matches, &_sites).await {
                    Ok(model) => model,
                    Err(e) => {
                        error!("{:?} {:?}", file, e);
                        continue;
                    }
                };

                debug!("{}: {:?}, {:?}", i, &file.file_name(), &model);

                match client.media_api().media_post(model, &file).await {
                    Ok(_) => {
                        info!("{}: uploaded", file.file_name().unwrap().to_string_lossy());
                    }
                    Err(e) => {
                        error!("{:?} {:?}", file, e);
                        continue;
                    }
                };
                if let Some(out_path) = &_out_folder {
                    let new_file = out_path.join(file.file_name().unwrap());
                    match std::fs::rename(&file,  &new_file) {
                        Ok(_) => {}
                        Err(e) => {error!("Failed to move {:?} to {:?} due to {:?}", file, new_file, e);}
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// DragonHorde Server to upload to
    #[arg(short, long, value_name = "http://localhost:8080/v1")]
    server: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create match database
    Init {
        /// Force database creation
        #[arg(short, long, default_value = "false")]
        force: bool,
        /// Fuzzysearch input csv
        #[arg(short, long)]
        csv: PathBuf,
    },
    Run {
        /// Directory or file to import
        #[arg(short, long, value_name = "PATH")]
        r#in: PathBuf,
        /// Where to move successfully uploaded files
        #[arg(short, long, value_name = "PATH")]
        r#out: Option<PathBuf>,
    },
}

#[tokio::main(worker_threads = 32)]
async fn main() {
    let args = Args::parse();

    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .filter_module("html5ever", LevelFilter::Warn)
        .filter_module("selectors", LevelFilter::Warn)
        .filter_module("sqlx", LevelFilter::Warn)
        .filter_module("hyper_util", LevelFilter::Warn)
        .filter_module("reqwest", LevelFilter::Warn)
        .init();

    let mut home = std::path::PathBuf::from(std::env::var("HOME").unwrap());
    home.push(".dragonhorde");

    let config_file = home.join(".dragonhorde_importer.toml");

    let mut builder = Config::builder()
        .set_default("home", home.to_str())
        .unwrap()
        .set_default("db", "db.sqlite")
        .unwrap()
        .add_source(File::from(config_file).required(false))
        .set_override_option("server", args.server)
        .unwrap();

    let settings = match builder.build() {
        Ok(config) => config,
        Err(e) => {
            error!("{}", e);
            panic!("{}", e)
        }
    };

    match &args.command {
        Some(Commands::Init { force, csv }) => {
            match create_db(&settings, force, csv).await {
                Ok(_) => {
                    info!("Database imported");
                }
                Err(e) => {
                    error!("{}", e);
                    panic!("{}", e)
                }
            };
        }
        Some(Commands::Run { r#in, r#out }) => {
            run(&settings, r#in, r#out).await;
        }
        _ => {}
    }
}
