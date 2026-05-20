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

#[derive(Debug,Clone)]
pub struct ApiResponse<T> {
    pub value: T,
    pub text: String,
}

impl Api {
    pub async fn get_video(&self, video_id: &VideoId) -> anyhow::Result<ApiResponse<PaginatedResponse<VideoItem>>> {
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
        let response = ApiResponse { value, text };
        Ok(response)
    }

    pub async fn get_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<ApiResponse<PaginatedResponse<PlaylistItem>>> {
        const MAX_RESULTS: usize = 50;
        let url = format!("{0}/playlistItems?part=contentDetails&playlistId={1}&key={2}&maxResults={3}", &self.base_url, playlist_id, self.api_key, MAX_RESULTS);
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
        let response = ApiResponse { value, text };
        Ok(response)
    }
}

