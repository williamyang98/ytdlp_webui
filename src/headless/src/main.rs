use app::database::{AudioExtension, VideoId, TranscodeKey};
use std::sync::Arc;
use app::app::App;
use app::app_config::AppConfig;
use clap::Parser;
use std::path::PathBuf;

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
    let app = App::new(app_config)?;
    let app = Arc::new(app);

    let video_id = VideoId::try_new("ILh0zEfqSQM")?;
    let key = TranscodeKey {
        video_id: video_id.clone(),
        audio_ext: AudioExtension::WEBM,
    };

    let worker = app.start_transcode(&key).await?;
    log::info!("start_transcode={0:?}", worker.get_status());
    worker.wait_busy();
    let state = worker.get_state();
    log::info!("transcode_state={state:?}");


    Ok(())
}
