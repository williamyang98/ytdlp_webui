use anyhow::Context;
use github_api::{GithubApi, DateTimeRfc3339};
use futures_util::StreamExt;
use regex::Regex;
use std::cmp::Ordering;
use std::path::Path;
#[derive(Clone,Debug)]
pub struct TaggedRelease {
    pub tag_name: String,
    pub asset_name: String,
    pub filesize_bytes: u64,
    pub browser_download_url: String,
    pub created_at: DateTimeRfc3339,
    pub updated_at: DateTimeRfc3339,
}

#[derive(Clone,Debug)]
pub enum TaggedFilename {
    String(String),
    Regex(Regex),
}

impl TaggedFilename {
    pub fn compare(&self, other: &str) -> bool {
        match self {
            Self::String(s) => s.as_str().cmp(other) == Ordering::Equal,
            Self::Regex(r) => r.is_match_at(other, 0),
        }
    }
}

#[derive(Clone,Debug)]
pub enum Tag {
    Latest,
    Fixed(String),
}

#[derive(Clone,Debug)]
pub struct TaggedAsset {
    pub owner: String,
    pub repo: String,
    pub tag: Tag,
    pub filename: TaggedFilename,
}

pub async fn get_tagged_release(api: &GithubApi, file: &TaggedAsset) -> anyhow::Result<Vec<TaggedRelease>> {
    let github_release = match &file.tag {
        Tag::Latest => api.get_latest_release(&file.owner, &file.repo).await?,
        Tag::Fixed(tag) => api.get_tagged_release(&file.owner, &file.repo, tag).await?,
    };
    let mut releases: Vec<TaggedRelease> = vec![];
    for asset in &github_release.assets {
        if file.filename.compare(&asset.name) {
            let release = TaggedRelease {
                tag_name: github_release.tag_name.clone(),
                asset_name: asset.name.clone(),
                filesize_bytes: asset.size,
                browser_download_url: asset.browser_download_url.clone(),
                created_at: asset.created_at.clone(),
                updated_at: asset.updated_at.clone(),
            };
            releases.push(release);
        }
    }
    releases.sort_by_key(|e| std::cmp::Reverse(e.filesize_bytes));
    releases.sort_by_key(|e| std::cmp::Reverse(e.updated_at.0));
    Ok(releases)
}

pub async fn download_file(api: &GithubApi, tagged_asset: &TaggedAsset, output_path: &Path) -> anyhow::Result<()> {
    let releases = get_tagged_release(api, tagged_asset).await?;
    let release = releases.first().ok_or(anyhow::anyhow!("No releases available"))?;
    log::info!("Selected release: tag='{0}' name='{1}'", &release.tag_name, &release.asset_name);

    let client = reqwest::Client::new();
    let response = client
        .get(release.browser_download_url.as_str())
        .send()
        .await?;

    let total_size_bytes = response
        .content_length()
        .ok_or(anyhow::anyhow!("Missing content length for download"))?;

    use indicatif::{ProgressBar, ProgressStyle, HumanBytes};
    log::info!("Total size of download: {0} bytes", HumanBytes(total_size_bytes));
    let progress_bar = ProgressBar::new(total_size_bytes);
    let progress_bar_style = ProgressStyle::default_bar()
        .template("{wide_bar:.green} {bytes}/{total_bytes} ({bytes_per_sec}) [{eta}]")
        .expect("Progress style template failed")
        .progress_chars("#>-");
    progress_bar.set_style(progress_bar_style);

    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(output_path).context("Failed to open output path")?;

    let mut total_downloaded_bytes: u64 = 0;
    let mut stream_bytes = response.bytes_stream();
    while let Some(chunk_response) = stream_bytes.next().await {
        let chunk = chunk_response?;
        file.write_all(&chunk)?;
        let chunk_size_bytes = chunk.len();
        total_downloaded_bytes += chunk_size_bytes as u64;
        progress_bar.set_position(total_downloaded_bytes);
    }
    drop(file);
    progress_bar.finish();
    Ok(())
}
