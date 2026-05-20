use crate::schema::{YoutubeApiKey, YoutubeMetadata, YoutubeVideoId};
use anyhow::Context;
use reqwest::Client;

pub static BASE_URL: &str = "https://www.googleapis.com/youtube/v3";

#[derive(Clone,Debug)]
pub struct YoutubeApi {
    pub client: Client,
    pub base_url: String,
    pub api_key: YoutubeApiKey,
}

impl Default for YoutubeApi {
    fn default() -> Self {
        dotenvy::dotenv().context("Failed to load .env file in default constructor").unwrap();
        let client = Client::new();
        let base_url: String = BASE_URL.to_owned();
        let api_key = std::env::var("YOUTUBE_API_KEY").context("Missing YOUTUBE_API_KEY in environment variables on default constructor").unwrap();
        let api_key: YoutubeApiKey = api_key.as_str().try_into().unwrap();
        Self {
            client,
            base_url,
            api_key,
        }
    }
}

impl YoutubeApi {
    pub async fn get_video_metadata(&self, video_id: &YoutubeVideoId) -> anyhow::Result<YoutubeMetadata> {
        let url = format!("{0}/videos?part=snippet,contentDetails&id={1}&key={2}", &self.base_url, video_id, self.api_key);
        let response = self.client
            .get(url)
            .send()
            .await?;
        let text = response
            .error_for_status()?
            .text()
            .await.context("Getting body from response")?;
        let metadata: YoutubeMetadata = serde_json::from_str(text.as_str())
            .with_context(|| format!("Failed to parse body into json: {text}"))?;
        Ok(metadata)
    }
}

