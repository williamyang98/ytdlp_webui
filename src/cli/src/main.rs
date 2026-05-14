use std::cmp::Ordering;
use std::path::PathBuf;
use app::github_api::{DateTimeRfc3339, get_github_releases};
use app::app::AppConfig;
use anyhow::Context;
use clap::Parser;
use serde::Serialize;
use futures_util::StreamExt;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Maximum number of transcode threads
    #[arg(long, default_value_t = 0)]
    total_transcode_threads: usize,
    /// Data folder
    #[arg(long, default_value = "./data", value_parser = validate_is_directory_empty_or_exists)]
    data_folder: PathBuf,
    /// Static website folder
    #[arg(long, default_value = "./static", value_parser = validate_is_directory_empty_or_exists)]
    static_folder: PathBuf,
}

fn validate_is_directory_empty_or_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && !path.is_dir() {
        Err("Cannot write to existing path that is not a directory".into())
    } else {
        Ok(path)
    }
}

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
    releases.sort_by_key(|e| std::cmp::Reverse(e.updated_at.0));
    Ok(releases)
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let total_transcode_threads: usize = match args.total_transcode_threads {
        0 => std::thread::available_parallelism().map(|v| v.get()).unwrap_or(1),
        x => x,
    };

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "INFO");
    }
    env_logger::init();

    let mut app_config = AppConfig::new(&args.data_folder, &args.static_folder)?;
    app_config.total_transcode_threads = total_transcode_threads;

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

    let ytdlp_binary_path = app_config.binaries_folder.join("yt-dlp.exe");
    let mut file = File::create(&ytdlp_binary_path).context("Failed to open ytdlp binary path")?;

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
