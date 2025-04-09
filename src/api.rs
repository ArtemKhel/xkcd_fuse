use std::fmt::{Display, Formatter};

use log::info;
use reqwest::Url;
use serde::Deserialize;

use crate::xkcd::Xkcd;

lazy_static::lazy_static!(
    static ref XKCD_URL: Url = Url::parse("https://xkcd.com/").unwrap();
);

const JSON: &str = "info.0.json";

#[derive(Debug, Deserialize)]
pub struct XkcdApiResponse {
    pub num: u32,
    pub title: String,
    pub safe_title: String,
    #[serde(rename = "img")]
    pub image_url: String,
    pub alt: String,
    pub transcript: String,
    pub link: String,

    pub year: String,
    pub month: String,
    pub day: String,

    pub news: String,
}

impl Display for XkcdApiResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "Xkcd #{}: {}", self.num, self.title,) }
}

pub fn fetch_latest(client: &reqwest::blocking::Client) -> Result<XkcdApiResponse, anyhow::Error> {
    info!("Fetching latest xkcd");
    let url = XKCD_URL.join("info.0.json")?;
    let resp = client.get(url).send()?;
    let comic: XkcdApiResponse = resp.json()?;
    Ok(comic)
}

pub fn fetch_xkcd(client: &reqwest::blocking::Client, num: u32) -> Result<XkcdApiResponse, anyhow::Error> {
    info!("Fetching xkcd {}", num);
    let mut url = XKCD_URL.clone();
    url.path_segments_mut().unwrap().extend([&num.to_string(), JSON]);
    let resp = client.get(url).send()?;
    let comic: XkcdApiResponse = resp.json()?;
    Ok(comic)
}

pub fn fetch_image(client: &reqwest::blocking::Client, comic: &Xkcd) -> Result<Vec<u8>, anyhow::Error> {
    info!("Fetching image for xkcd {}", comic.num);
    let resp = client.get(comic.image_url.clone()).send()?;
    let bytes = resp.bytes()?.to_vec();
    Ok(bytes)
}
