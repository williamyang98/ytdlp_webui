use std::{collections::HashMap, sync::Arc};
use dashmap::DashMap;
use serde::{Serialize,Deserialize};
use crate::database::VideoId;

pub type MetadataCache = Arc<DashMap<VideoId, Arc<Metadata>>>;

pub fn get_metadata_url(video_id: &str) -> String {
    const URL: &str = "https://www.googleapis.com/youtube/v3/videos";
    const PARTS: &str = "snippet,contentDetails";
    const API_KEY: &str = "AIzaSyDkmFSz9gH9slSnonGjs8TZEjtAKS4e9cg";
    format!("{URL}?part={PARTS}&id={video_id}&key={API_KEY}")
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct ContentDetails {
    pub duration: String,
    pub dimension: String,
    pub definition: String,
    pub caption: String,
    #[serde(rename="licensedContent")]
    pub licensed_content: bool,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Snippet {
    #[serde(rename="publishedAt")]
    pub published_at: String,
    #[serde(rename="channelId")]
    pub channel_id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub thumbnails: HashMap<String, Thumbnail>,
    #[serde(rename="channelTitle")]
    pub channel_title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename="categoryId")]
    pub category_id: String,

}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Item {
    pub id: String,
    pub etag: String,
    pub kind: String,
    pub snippet: Snippet,
    #[serde(rename="contentDetails")]
    pub content_details: ContentDetails,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct PageInfo {
    #[serde(rename="totalResults")]
    pub total_results: usize,
    #[serde(rename="resultsPerPage")]
    pub results_per_page: usize,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Metadata {
    pub kind: String,
    pub etag: String,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(rename="pageInfo")]
    pub page_info: PageInfo,
}
