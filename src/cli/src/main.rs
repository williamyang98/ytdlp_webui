use std::cmp::Ordering;
use app::github_api::{DateTimeRfc3339, get_github_releases};
use serde::Serialize;
use futures_util::StreamExt;

#[derive(Clone,Debug,Serialize)]
pub struct YtdlpRelease {
    pub tag_name: String,
    pub browser_download_url: String,
    pub created_at: DateTimeRfc3339,
    pub updated_at: DateTimeRfc3339,
}

pub async fn get_ytdlp_releases() -> anyhow::Result<Vec<YtdlpRelease>> {
    let github_releases = get_github_releases("yt-dlp", "yt-dlp", 10, 1).await?;
    let mut releases: Vec<YtdlpRelease> = vec![];
    for github_release in &github_releases {
        for asset in &github_release.assets {
            if asset.name.as_str().cmp("yt-dlp.exe") == Ordering::Equal {
                let release = YtdlpRelease {
                    tag_name: github_release.tag_name.clone(),
                    browser_download_url: asset.browser_download_url.clone(),
                    created_at: asset.created_at.clone(),
                    updated_at: asset.updated_at.clone(),
                };
                releases.push(release);
            }
        }
    }
    releases.sort_by(|a, b| b.updated_at.0.cmp(&a.updated_at.0));
    Ok(releases)
}

#[pollster::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let releases = get_ytdlp_releases().await?;
    let release = releases.first().ok_or(anyhow::anyhow!("No releases available"))?;
    log::info!("Found release: {:?}", release);

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
    let mut file = File::create("./bin/yt-dlp.exe")?;

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
