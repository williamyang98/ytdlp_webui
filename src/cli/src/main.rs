use anyhow::Context;
use github_api::{GithubApi, DateTimeRfc3339};
use clap::Parser;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use regex::Regex;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Binaries folder
    #[arg(long, default_value = "./bin", value_parser = validate_is_directory_empty_or_exists)]
    binaries_folder: PathBuf,
}

fn validate_is_directory_empty_or_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && !path.is_dir() {
        Err("Cannot write to existing path that is not a directory".into())
    } else {
        Ok(path)
    }
}

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
pub struct TaggedAsset {
    pub owner: String,
    pub repo: String,
    pub filename: TaggedFilename,
}

async fn get_tagged_release(api: &GithubApi, file: &TaggedAsset) -> anyhow::Result<Vec<TaggedRelease>> {
    let total_per_page = 10;
    let page = 1;
    let github_releases = api.get_releases(&file.owner, &file.repo, total_per_page, page).await?;
    let mut releases: Vec<TaggedRelease> = vec![];
    for github_release in &github_releases {
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
    }
    releases.sort_by_key(|e| std::cmp::Reverse(e.filesize_bytes));
    releases.sort_by_key(|e| std::cmp::Reverse(e.updated_at.0));
    Ok(releases)
}

async fn download_file(api: &GithubApi, tagged_asset: &TaggedAsset, output_path: &Path) -> anyhow::Result<()> {
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

async fn download_files(api: &GithubApi, binaries_folder: &Path) -> anyhow::Result<()> {
    download_file(
        api,
        &TaggedAsset {
            owner: "yt-dlp".to_owned(),
            repo: "yt-dlp".to_owned(),
            filename: TaggedFilename::String("yt-dlp.exe".to_owned()),
        },
        &binaries_folder.join("yt-dlp.exe"),
    ).await?;

    download_file(
        api,
        &TaggedAsset {
            owner: "ip7z".to_owned(),
            repo: "7zip".to_owned(),
            filename: TaggedFilename::String("7zr.exe".to_owned()),
        },
        &binaries_folder.join("7zr.exe"),
    ).await?;


    lazy_static! {
        static ref EXTRA_7ZIP_FILENAME_REGEX: Regex = Regex::new(
            r"7z.*extra.*\.7z"
        ).unwrap();
    }

    download_file(
        api,
        &TaggedAsset {
            owner: "ip7z".to_owned(),
            repo: "7zip".to_owned(),
            filename: TaggedFilename::Regex(EXTRA_7ZIP_FILENAME_REGEX.clone()),
        },
        &binaries_folder.join("7z-extra.7z"),
    ).await?;

    lazy_static! {
        static ref FFMPEG_WIN64_FILENAME_REGEX: Regex = Regex::new(
            r"ffmpeg.*win64.*gpl.*\.zip"
        ).unwrap();
    }

    // https://www.ffmpeg.org/download.html#build-windows
    download_file(
        api,
        &TaggedAsset {
            owner: "BtbN".to_owned(),
            repo: "FFmpeg-Builds".to_owned(),
            filename: TaggedFilename::Regex(FFMPEG_WIN64_FILENAME_REGEX.clone()),
        },
        &binaries_folder.join("ffmpeg.zip"),
    ).await?;

    Ok(())
}

async fn extract_files(binaries_folder: &Path) -> anyhow::Result<()> {
    use std::process::Command;
    let zip_minimal_executable_path = binaries_folder.join("7zr.exe")
        .canonicalize()
        .context("Failed to find 7zip minimal executable path")?;
    let zip_extras_archive_path = binaries_folder.join("7z-extra.7z")
        .canonicalize()
        .context("Failed to find 7zip extra archive path")?;

    let output = Command::new(&zip_minimal_executable_path)
        .arg("e")
        .arg("-y")
        .arg(&zip_extras_archive_path)
        .arg("7za.exe")
        .current_dir(binaries_folder)
        .output()
        .context("Failed to unzip 7zip minimal executable")?;
    if !output.status.success() {
        let stderr = str::from_utf8(&output.stderr).expect("Failed to read stderr as string");
        return Err(anyhow::anyhow!("Failed to unzip 7zip minimal executable: {stderr}"));
    }
    log::info!("Unzipped 7zip extras executable");

    let zip_extras_executable_path = binaries_folder.join("7za.exe")
        .canonicalize()
        .context("Failed to find 7zip extras executable path")?;
    let ffmpeg_archive_path = binaries_folder.join("ffmpeg.zip")
        .canonicalize()
        .context("Failed to find ffmpeg archive path")?;
    let output = Command::new(&zip_extras_executable_path)
        .arg("e")
        .arg("-y")
        .arg(&ffmpeg_archive_path)
        .arg("*/bin/ffmpeg.exe")
        .current_dir(binaries_folder)
        .output()
        .context("Failed to unzip ffmpeg minimal executable")?;
    if !output.status.success() {
        let stderr = str::from_utf8(&output.stderr).expect("Failed to read stderr as string");
        return Err(anyhow::anyhow!("Failed to unzip ffmpeg executable: {stderr}"));
    }
    log::info!("Unzipped ffmpeg executable");

    Ok(())
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "INFO");
    }
    env_logger::init();

    std::fs::create_dir_all(&args.binaries_folder)
        .context("Failed to open binaries folder")?;

    let api = GithubApi::default();
    download_files(&api, &args.binaries_folder).await?;
    extract_files(&args.binaries_folder).await?;

    Ok(())
}
