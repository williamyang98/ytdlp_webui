use crate::schema::Release;
use anyhow::Context;
use reqwest::header::{HeaderMap, USER_AGENT};

pub static BASE_URL: &str = "https://api.github.com";

pub async fn get_github_releases(owner: &str, repo: &str, per_page: u32, page: u32) -> anyhow::Result<Vec<Release>> {
    let url = format!("{BASE_URL}/repos/{owner}/{repo}/releases?per_page={per_page}&page={page}");
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "ytdlp_webui".parse().unwrap());
    headers.insert("X-GitHub-Api-Version", "2026-03-10".parse().unwrap());
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .headers(headers)
        .send().await?;
    let data = response.text().await.context("Getting body from response")?;
    let releases: Vec<Release> = serde_json::from_str(data.as_str()).context("Parsing body into json")?;
    Ok(releases)
}
