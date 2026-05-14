use serde::{Serialize,Deserialize};

#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(transparent)]
pub struct DateTimeRfc3339(#[serde(with = "time::serde::rfc3339")] pub time::OffsetDateTime);

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Release {
    pub url: String,
    pub html_url: String,
    pub id: u64,
    pub author: Author,
    pub tag_name: String,
    pub target_commitish: String,
    pub name: String,
    pub prerelease: bool,
    pub created_at: DateTimeRfc3339,
    pub updated_at: DateTimeRfc3339,
    pub published_at: DateTimeRfc3339,
    pub assets: Vec<Asset>,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Author {
    pub login: String,
    pub id: u64,
    pub avatar_url: String,
    pub html_url: String,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub id: u64,
    pub label: String,
    pub browser_download_url: String,
    pub digest: String,
    pub uploader: Author,
    pub content_type: String,
    pub size: u64,
    pub created_at: DateTimeRfc3339,
    pub updated_at: DateTimeRfc3339,
    pub download_count: u64,
}

pub fn get_github_releases_url(owner: &str, repo: &str, per_page: u32, page: u32) -> String {
    const URL: &str = "https://api.github.com/repos";
    format!("{URL}/{owner}/{repo}/releases?per_page={per_page}&page={page}")
}

pub async fn get_github_releases(owner: &str, repo: &str, per_page: u32, page: u32) -> anyhow::Result<Vec<Release>> {
    let url = get_github_releases_url(owner, repo, per_page, page);
    use reqwest::header::{HeaderMap, USER_AGENT};
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "ytdlp_webui".parse().unwrap());
    headers.insert("X-GitHub-Api-Version", "2026-03-10".parse().unwrap());
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .headers(headers)
        .send().await?;
    let data = response.text().await?;
    let releases: Vec<Release> = serde_json::from_str(data.as_str())?;
    Ok(releases)
}
