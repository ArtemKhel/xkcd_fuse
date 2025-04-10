use std::{cmp::min, collections::HashSet, num::NonZeroU32, path::PathBuf};
use std::sync::{Arc,Mutex};

use indicatif::{ProgressBar, ProgressStyle};
use futures::{StreamExt, stream::FuturesUnordered};
use governor::{Quota, RateLimiter};
use log::{error, info, warn};
use rusqlite::Connection;

use crate::{api, db, xkcd::Xkcd};

pub struct XkcdStorageConfig {
    pub db_path: PathBuf,
}

pub trait Storage {
    fn get_stored_ids(&self) -> Vec<u32>;
    // fn get_latest(&self) -> Option<Xkcd>;
    fn get_meta(&self, num: u32) -> Option<Xkcd>;
    fn get_image(&self, num: u32) -> Option<Vec<u8>>;
    fn get_xkcd(&self, num: u32) -> Option<(Xkcd, Vec<u8>)>;
    fn get_image_size(&self, num: u32) -> Option<u64>;
}

#[derive(Debug)]
pub struct XkcdStorage {
    db_conn: Connection,
    http_client: reqwest::Client,
}

unsafe impl Send for XkcdStorage {}
unsafe impl Sync for XkcdStorage {}

impl XkcdStorage {
    pub fn new(config: XkcdStorageConfig) -> Self {
        let db_conn = Connection::open(&config.db_path)
            .unwrap_or_else(|e| panic!("Failed to open database at {}: {e}", config.db_path.display()));

        db::db_init(&db_conn).unwrap_or_else(|e| panic!("Failed to initialize database: {e}"));

        let http_client = reqwest::Client::new();

        Self { db_conn, http_client }
    }

    pub async fn ensure_range(&self, start: u32, end: u32) -> Result<(), ()> {
        const RPS: u32 = 25;
        let limiter = RateLimiter::direct(Quota::per_second(NonZeroU32::new(RPS).unwrap()));
        let mut tasks = FuturesUnordered::new();

        let latest = self.get_latest().await.ok_or(())?;
        let end = min(end, latest.num);
        let start = min(start, end);
        info!("Fetching xkcd range {}-{}", start, end);

        let ids: HashSet<_> = self.get_stored_ids().into_iter().collect();
        let missing = (start..=end).filter(|num| !ids.contains(num)).collect::<Vec<_>>();

        let missing_len = missing.len();
        let progress_bar = Arc::new(Mutex::new(ProgressBar::new(missing_len as u64)));

        for num in missing.into_iter() {
            let permit = limiter.until_ready();
            let bar = Arc::clone(&progress_bar);
            let future = async move {
                permit.await;
                self.get_xkcd(num).await;
                bar.lock().unwrap().inc(1);
            };
            tasks.push(future);
        }

        while tasks.next().await.is_some() {}
        progress_bar.lock().unwrap().finish_with_message("Done!");
        Ok(())
    }

    async fn get_latest(&self) -> Option<Xkcd> {
        let latest = api::fetch_latest(&self.http_client)
            .await
            .map(|xkcd| Some(xkcd.into()))
            .unwrap_or_else(|e| {
                error!("Failed to get latest xkcd: {e}");
                None
            })?;
        db::insert_meta(&self.db_conn, &latest).unwrap_or_else(|e| {
            error!("Failed to insert latest xkcd into DB: {e}");
        });
        let _ = self.get_image(latest.num).await;
        Some(latest)
    }

    fn get_stored_ids(&self) -> Vec<u32> {
        db::get_stored_ids(&self.db_conn).unwrap_or_else(|e| {
            error!("Failed to get stored IDs: {e}");
            vec![]
        })
    }

    async fn get_meta(&self, num: u32) -> Option<Xkcd> {
        match db::get_meta(&self.db_conn, num) {
            Ok(xkcd) => {
                info!("Xkcd {num} already in DB");
                Some(xkcd)
            }
            Err(e) => {
                if let Some(rusqlite::Error::QueryReturnedNoRows) = e.downcast_ref::<rusqlite::Error>() {
                    info!("Xkcd {num} not in DB, fetching");
                    let xkcd = api::fetch_xkcd(&self.http_client, num)
                        .await
                        .map(|xkcd| xkcd.into())
                        .unwrap_or_else(|e| {
                            error!("Failed to fetch xkcd {num}: {e}");
                            None
                        })?
                        .into();
                    let _ = db::insert_meta(&self.db_conn, &xkcd).map_err(|e| {
                        error!("Failed to insert xkcd into DB: {e}");
                    });
                    Some(xkcd)
                } else {
                    error!("Failed to get xkcd {num}: {e}");
                    None
                }
            }
        }
    }

    async fn get_image(&self, num: u32) -> Option<Vec<u8>> {
        match db::get_image(&self.db_conn, num) {
            Ok(img) => {
                info!("Image for xkcd {num} already in DB");
                Some(img)
            }
            Err(e) => {
                if let Some(rusqlite::Error::QueryReturnedNoRows) = e.downcast_ref::<rusqlite::Error>() {
                    info!("Image for xkcd {num} not in DB, fetching");
                    let meta = self.get_meta(num).await?;
                    let img = api::fetch_image(&self.http_client, &meta)
                        .await
                        .map_err(|e| {
                            error!("Failed to fetch image for xkcd {num}: {e:?}");
                        })
                        .ok()?;
                    let _ = db::insert_image(&self.db_conn, num, &img).map_err(|e| {
                        error!("Failed to insert image into DB for xkcd {num}: {e:?}");
                    });
                    Some(img)
                } else {
                    error!("Failed to get image from DB for xkcd {num}: {e}");
                    None
                }
            }
        }
    }

    async fn get_xkcd(&self, num: u32) -> Option<(Xkcd, Vec<u8>)> {
        let meta = self.get_meta(num).await?;
        let image = self.get_image(num).await?;
        Some((meta, image))
    }

    fn get_image_size(&self, num: u32) -> Option<u64> {
        db::get_image_size(&self.db_conn, num).map(Some).unwrap_or_else(|e| {
            error!("Failed to get image size for xkcd {num}: {e}");
            None
        })
    }
}

impl From<XkcdStorageConfig> for XkcdStorage {
    fn from(config: XkcdStorageConfig) -> Self { Self::new(config) }
}

pub struct BlockingXkcdStorage {
    storage: XkcdStorage,
    rt: tokio::runtime::Runtime,
}

impl BlockingXkcdStorage {
    pub fn ensure_range(&self, start: u32, end: u32) -> Result<(), ()> {
        self.rt.block_on(self.storage.ensure_range(start, end))
    }
}

impl From<XkcdStorage> for BlockingXkcdStorage {
    fn from(storage: XkcdStorage) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        Self { storage, rt }
    }
}
impl Storage for BlockingXkcdStorage {
    fn get_stored_ids(&self) -> Vec<u32> { self.storage.get_stored_ids() }

    fn get_meta(&self, num: u32) -> Option<Xkcd> { self.rt.block_on(self.storage.get_meta(num)) }

    fn get_image(&self, num: u32) -> Option<Vec<u8>> { self.rt.block_on(self.storage.get_image(num)) }

    fn get_xkcd(&self, num: u32) -> Option<(Xkcd, Vec<u8>)> { self.rt.block_on(self.storage.get_xkcd(num)) }

    fn get_image_size(&self, num: u32) -> Option<u64> { self.storage.get_image_size(num) }
}
