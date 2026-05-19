use serde::{Serialize,Deserialize};

// API schema for github api
// https://docs.github.com/en/rest/releases/releases?apiVersion=2026-03-10
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
    pub name: Option<String>,
    pub email: Option<String>,
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
    pub label: Option<String>,
    pub browser_download_url: String,
    pub digest: Option<String>,
    pub uploader: Option<Author>,
    pub content_type: String,
    pub size: u64,
    pub created_at: DateTimeRfc3339,
    pub updated_at: DateTimeRfc3339,
    pub download_count: u64,
}


