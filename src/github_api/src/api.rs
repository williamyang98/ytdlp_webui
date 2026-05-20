use crate::schema::Release;
use anyhow::Context;
use reqwest::Client;
use reqwest::header::{HeaderMap, USER_AGENT};

pub static BASE_URL: &str = "https://api.github.com";

#[derive(Clone,Debug)]
pub struct GithubApi {
    pub client: Client,
    pub base_url: String,
    pub headers: HeaderMap,
}

impl Default for GithubApi {
    fn default() -> Self {
        let client = Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "ytdlp_webui".parse().unwrap());
        headers.insert("X-GitHub-Api-Version", "2026-03-10".parse().unwrap());
        let base_url: String = BASE_URL.to_owned();
        Self {
            client,
            base_url,
            headers,
        }
    }
}

impl GithubApi {
    pub async fn get_releases(&self, owner: &str, repo: &str, per_page: u32, page: u32) -> anyhow::Result<Vec<Release>> {
        let url = format!("{0}/repos/{owner}/{repo}/releases?per_page={per_page}&page={page}", &self.base_url);
        let response = self.client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await?;
        let text = response
            .error_for_status()?
            .text()
            .await.context("Getting body from response")?;
        let releases: Vec<Release> = serde_json::from_str(text.as_str())
            .with_context(|| format!("Failed to parse body into json: {text}"))?;
        Ok(releases)
    }
}

