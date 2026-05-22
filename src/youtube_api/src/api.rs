use crate::schema::{
    PaginatedResponse,
    PlaylistId, PlaylistItem,
    VideoId, VideoItem,
};
use lazy_static::lazy_static;
use regex::Regex;
use anyhow::Context;
use reqwest::Client;
use derive_more::{AsRef, Display, Into};
use std::collections::HashSet;

#[derive(Clone,Debug,Display,AsRef,Into)]
pub struct ApiKey(String);

impl TryInto<ApiKey> for &str {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ApiKey, Self::Error> {
        lazy_static! {
            static ref YOUTUBE_API_KEY_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9\\.\-\_]{24,}$").unwrap();
        }
        if !YOUTUBE_API_KEY_REGEX.is_match(self) {
            return Err(anyhow::anyhow!("Invalid youtube api key: {0}", self));
        }
        Ok(ApiKey(self.to_owned()))
    }
}


pub static BASE_URL: &str = "https://www.googleapis.com/youtube/v3";

#[derive(Clone,Debug)]
pub struct Api {
    pub client: Client,
    pub base_url: String,
    pub api_key: ApiKey,
}

impl Default for Api {
    fn default() -> Self {
        dotenvy::dotenv().context("Failed to load .env file in default constructor").unwrap();
        let client = Client::new();
        let base_url: String = BASE_URL.to_owned();
        let api_key = std::env::var("YOUTUBE_API_KEY").context("Missing YOUTUBE_API_KEY in environment variables on default constructor").unwrap();
        let api_key: ApiKey = api_key.as_str().try_into().unwrap();
        Self {
            client,
            base_url,
            api_key,
        }
    }
}

impl Api {
    pub async fn get_video(&self, video_id: &VideoId) -> anyhow::Result<VideoItem> {
        let url = format!("{0}/videos?part=snippet,contentDetails&id={1}&key={2}", &self.base_url, video_id, self.api_key);
        let response = self.client
            .get(url)
            .send()
            .await?;
        let text = response
            .error_for_status()?
            .text()
            .await.context("Getting body from response")?;
        let value: PaginatedResponse<VideoItem> = serde_json::from_str(text.as_str())
            .with_context(|| format!("Failed to parse body into json: {text}"))?;
        let Some(video) = value.items.first() else {
            return Err(anyhow::anyhow!("No data for video_id={0}", video_id));
        };
        Ok(video.clone())
    }

    pub async fn get_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<PlaylistItem>> {
        const MAX_RESULTS: usize = 50;
        let url = format!("{0}/playlistItems?part=contentDetails&playlistId={1}&key={2}&maxResults={3}", &self.base_url, playlist_id, self.api_key, MAX_RESULTS);
        let mut items: Vec<PlaylistItem> = vec![];
        let mut next_page_token: Option<String> = None;
        let mut explored_pages = HashSet::<String>::new();
        loop {
            let url = match next_page_token.as_ref() {
                None => url.clone(), // empty start
                Some(token) => { // page continue
                    let is_new_token = explored_pages.insert(token.clone());
                    if !is_new_token {
                        log::debug!("Stop iterating through playlist since we already received a previous page_token={0} for {1} total pages", token, explored_pages.len());
                        break;
                    }
                    format!("{url}&pageToken={token}")
                },
            };
            log::debug!("get_playlist: url={0}", url.as_str());
            let response = self.client
                .get(url)
                .send()
                .await?;
            let text = response
                .error_for_status()?
                .text()
                .await.context("Getting body from response")?;
            let value: PaginatedResponse<PlaylistItem> = serde_json::from_str(text.as_str())
                .with_context(|| format!("Failed to parse body into json: {text}"))?;
            items.extend_from_slice(value.items.as_slice());
            next_page_token = value.next_page_token.clone();
            if next_page_token.is_none() {
                break;
            }
        }
        Ok(items)
    }
}

