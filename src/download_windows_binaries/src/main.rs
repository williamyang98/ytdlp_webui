use anyhow::Context;
use github_api::GithubApi;
use clap::Parser;
use lazy_static::lazy_static;
use regex::Regex;
use std::path::PathBuf;
use std::process::Command;
use download_windows_binaries::{download_file, TaggedFilename, TaggedAsset, Tag};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Binaries folder
    #[arg(long, default_value = "./bin", value_parser = validate_is_directory_empty_or_exists)]
    binaries_folder: PathBuf,
    /// Clean reinstall
    #[arg(long)]
    clean: bool,
    /// Download latest yt-dlp
    #[arg(long)]
    download_latest_ytdlp: bool,
    /// Download latest ffmpeg
    #[arg(long)]
    download_latest_ffmpeg: bool,
}

fn validate_is_directory_empty_or_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && !path.is_dir() {
        Err("Cannot write to existing path that is not a directory".into())
    } else {
        Ok(path)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "INFO");
    }
    env_logger::init();

    if args.clean {
        log::info!("Performing a clean installation");
        std::fs::remove_dir_all(&args.binaries_folder)
            .context("Failed to clean binaries folder")?;
    }

    std::fs::create_dir_all(&args.binaries_folder)
        .context("Failed to create or open binaries folder")?;

    let api = GithubApi::default();
    let binaries_folder = args.binaries_folder;

    // download yt-dlp
    let filepath_ytdlp_executable = binaries_folder.join("yt-dlp.exe");
    if !filepath_ytdlp_executable.exists() || args.download_latest_ytdlp {
        let ytdlp_tag: Tag = if args.download_latest_ytdlp {
            log::info!("Downloading yt-dlp latest");
            Tag::Latest
        } else {
            let tag = "2026.06.09";
            log::info!("Downloading yt-dlp from tag '{tag}'");
            Tag::Fixed(tag.to_string())
        };
        download_file(
            &api,
            &TaggedAsset {
                owner: "yt-dlp".to_owned(),
                repo: "yt-dlp".to_owned(),
                tag: ytdlp_tag,
                filename: TaggedFilename::String("yt-dlp.exe".to_owned()),
            },
            &filepath_ytdlp_executable,
        ).await?;
    }

    // download 7z restricted
    let filepath_7zr = binaries_folder.join("7zr.exe");
    if !filepath_7zr.exists() {
        download_file(
            &api,
            &TaggedAsset {
                owner: "ip7z".to_owned(),
                repo: "7zip".to_owned(),
                tag: Tag::Fixed("26.01".to_owned()),
                filename: TaggedFilename::String("7zr.exe".to_owned()),
            },
            &filepath_7zr,
        ).await?;
    }

    // download 7z extras
    let filepath_7z_extra = binaries_folder.join("7z-extra.7z");
    if !filepath_7z_extra.exists() {
        lazy_static! {
            static ref EXTRA_7ZIP_FILENAME_REGEX: Regex = Regex::new(
                r"7z.*extra.*\.7z"
            ).unwrap();
        }
        download_file(
            &api,
            &TaggedAsset {
                owner: "ip7z".to_owned(),
                repo: "7zip".to_owned(),
                tag: Tag::Fixed("26.01".to_owned()),
                filename: TaggedFilename::Regex(EXTRA_7ZIP_FILENAME_REGEX.clone()),
            },
            &filepath_7z_extra,
        ).await?;
    }

    // download ffmpeg archive
    let filepath_ffmpeg_archive = binaries_folder.join("ffmpeg.zip");
    let mut ffmpeg_archive_changed = false;
    if !filepath_ffmpeg_archive.exists() || args.download_latest_ffmpeg {
        lazy_static! {
            static ref FFMPEG_WIN64_FILENAME_REGEX: Regex = Regex::new(
                r"ffmpeg.*win64.*gpl.*\.zip"
            ).unwrap();
        }

        // https://www.ffmpeg.org/download.html#build-windows
        let ffmpeg_tag: Tag = if args.download_latest_ffmpeg {
            log::info!("Downloading ffmpeg latest");
            Tag::Latest
        } else {
            let tag = "autobuild-2026-06-12-14-04";
            log::info!("Downloading ffmpeg from tag '{tag}'");
            Tag::Fixed(tag.to_string())
        };
        download_file(
            &api,
            &TaggedAsset {
                owner: "BtbN".to_owned(),
                repo: "FFmpeg-Builds".to_owned(),
                tag: ffmpeg_tag,
                filename: TaggedFilename::Regex(FFMPEG_WIN64_FILENAME_REGEX.clone()),
            },
            &filepath_ffmpeg_archive
        ).await?;
        ffmpeg_archive_changed = true;
    }

    // unzip 7za.exe (all) using 7zr.exe (restricted)
    let zip_minimal_executable_path = binaries_folder.join("7zr.exe")
        .canonicalize()
        .context("Failed to find 7zip minimal executable path")?;
    let zip_extras_archive_path = binaries_folder.join("7z-extra.7z")
        .canonicalize()
        .context("Failed to find 7zip extra archive path")?;
    let zip_extras_executable_path = binaries_folder.join("7za.exe");
    if !zip_extras_executable_path.exists() {
        let output = Command::new(&zip_minimal_executable_path)
            .arg("e")
            .arg("-y")
            .arg(&zip_extras_archive_path)
            .arg("7za.exe")
            .current_dir(&binaries_folder)
            .output()
            .context("Failed to unzip 7zip minimal executable")?;
        if !output.status.success() {
            let stderr = str::from_utf8(&output.stderr).expect("Failed to read stderr as string");
            return Err(anyhow::anyhow!("Failed to unzip 7zip minimal executable: {stderr}"));
        }
        log::info!("Unzipped 7zip extras executable");
    }
    let zip_extras_executable_path = zip_extras_executable_path
        .canonicalize()
        .context("Failed to find 7zip extras executable path")?;

    // unzip ffmpeg.exe from ffmpeg.zip
    let filepath_ffmpeg_archive = filepath_ffmpeg_archive
        .canonicalize()
        .context("Failed to find ffmpeg archive path")?;
    if ffmpeg_archive_changed {
        let output = Command::new(&zip_extras_executable_path)
            .arg("e")
            .arg("-y")
            .arg(&filepath_ffmpeg_archive)
            .arg("*/bin/ffmpeg.exe")
            .current_dir(&binaries_folder)
            .output()
            .context("Failed to unzip ffmpeg minimal executable")?;
        if !output.status.success() {
            let stderr = str::from_utf8(&output.stderr).expect("Failed to read stderr as string");
            return Err(anyhow::anyhow!("Failed to unzip ffmpeg executable: {stderr}"));
        }
        let _ffmpeg_executable_path = binaries_folder.join("ffmpeg.exe")
            .canonicalize()
            .context("Failed to find ffmpeg executable path")?;
        log::info!("Unzipped ffmpeg executable");
    }

    Ok(())
}
