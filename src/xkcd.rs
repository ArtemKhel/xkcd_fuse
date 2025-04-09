use std::{fmt::Display, time::SystemTime};

use chrono::{NaiveDate, TimeZone, Utc};
use log::{error, info};
use serde::Deserialize;

use crate::api::XkcdApiResponse;

#[derive(Debug, Deserialize)]
pub struct Xkcd {
    pub num: u32,
    pub title: String,
    pub safe_title: String,
    pub image_url: String,
    pub alt: String,
    pub transcript: String,
    pub link: String,
    pub release_date: NaiveDate,
}

impl Xkcd {
    pub fn release_date_as_timestamp(&self) -> SystemTime {
        SystemTime::from(Utc.from_utc_datetime(&self.release_date.and_hms_opt(0, 0, 0).unwrap()))
    }
}

impl Display for Xkcd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Xkcd #{}: {}", self.num, self.title)
    }
}

fn parse_date(year: &str, month: &str, day: &str) -> NaiveDate {
    let year = year.parse().unwrap_or_else(|e| {
        error!("Failed to parse year: {e}");
        0
    });
    let month = month.parse().unwrap_or_else(|e| {
        error!("Failed to parse month: {e}");
        0
    });
    let day = day.parse().unwrap_or_else(|e| {
        error!("Failed to parse day: {e}");
        0
    });
    NaiveDate::from_ymd_opt(year, month, day).expect("Failed to parse date")
}

impl From<XkcdApiResponse> for Xkcd {
    fn from(value: XkcdApiResponse) -> Self {
        let release_date = parse_date(&value.year, &value.month, &value.day);
        if !value.news.is_empty() {
            info!("Xkcd {} has news: {}", value.num, value.news);
        }
        Self {
            num: value.num,
            title: value.title,
            safe_title: value.safe_title,
            image_url: value.image_url,
            alt: value.alt,
            transcript: value.transcript,
            link: value.link,
            release_date,
        }
    }
}
