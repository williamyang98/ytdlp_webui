use app::database::{AudioExtension, TranscodeKey};
use std::sync::Arc;
use app::app::App;
use app::app_config::AppConfig;
use clap::Parser;
use std::path::PathBuf;
use youtube_api::{VideoId, VideoIdError};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Video id
    #[arg(default_value = "dQw4w9WgXcQ", value_parser = validate_video_id)]
    video_id: VideoId,
    /// Audio extension
    #[arg(long, default_value = AudioExtension::MP3.as_str(), value_parser = validate_audio_extension)]
    audio_extension: AudioExtension,
    /// Download video_info
    #[arg(long)]
    download_video_info: bool,
    /// Maximum number of transcode threads
    #[arg(long, default_value_t = 0)]
    total_transcode_threads: usize,
    /// Data folder
    #[arg(long, default_value = "./data", value_parser = validate_is_directory_empty_or_exists)]
    data_folder: PathBuf,
    /// Static website folder
    #[arg(long, default_value = "./static", value_parser = validate_is_directory_empty_or_exists)]
    static_folder: PathBuf,
    /// Environment file
    #[arg(long, default_value = ".env", value_parser = validate_is_file_exists)]
    env_file: PathBuf,
}

fn validate_video_id(s: &str) -> Result<VideoId, String> {
    s.try_into().map_err(|e: VideoIdError| e.to_string())
}

fn validate_audio_extension(s: &str) -> Result<AudioExtension, String> {
    AudioExtension::try_from(s).map_err(|e| e.to_string())
}

fn validate_is_directory_empty_or_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && !path.is_dir() {
        Err("Cannot write to existing path that is not a directory".into())
    } else {
        Ok(path)
    }
}

fn validate_is_file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if !path.is_file() {
        Err("File is missing".into())
    } else {
        Ok(path)
    }
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
    log::info!("Downloading {0}.{1}", args.video_id.as_str(), args.audio_extension.as_str());

    let mut app_config = AppConfig::new(&args.env_file, &args.data_folder, &args.static_folder)?;
    app_config.total_transcode_threads = total_transcode_threads;
    let app = App::new(app_config)?;
    let app = Arc::new(app);
    let key = TranscodeKey {
        video_id: args.video_id.clone(),
        audio_ext: args.audio_extension,
    };

    let download_worker = app.start_download(&args.video_id)?;
    log::info!("download_worker: status={0:?}", download_worker.get_status());
    let video_info = if args.download_video_info {
        let video_info = app.get_youtube_video(&args.video_id).await?;
        log::info!("youtube_video_info: title={0:?}", video_info.snippet.title.as_str());
        Some(video_info)
    } else {
        None
    };
    let transcode_worker = app.start_transcode(&key, video_info)?;
    log::info!("start_transcode: status={0:?}", transcode_worker.get_status());
    transcode_worker.wait_busy();
    let state = transcode_worker.get_state();
    log::info!("transcode_state={state:?}");


    Ok(())
}
