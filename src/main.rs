use std::path::PathBuf;
use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
use ytdlp_server::{
    app::{AppConfig, AppState},
    routes,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Url of server
    #[arg(long, default_value = "0.0.0.0")]
    url: String,
    /// Port of server
    #[arg(long, default_value_t = 8080)]
    port: u16,
    /// Maximum number of transcode threads
    #[arg(long, default_value_t = 0)]
    total_transcode_threads: usize,
    /// Maximum number of worker threads
    #[arg(long, default_value_t = 0)]
    total_worker_threads: usize,
    /// ffmpeg binary for transcoding between formats
    #[arg(long)]
    #[cfg_attr(windows, arg(default_value = Some("./bin/ffmpeg.exe")))]
    #[cfg_attr(unix, arg(default_value = Some("ffmpeg")))]
    ffmpeg_binary_path: Option<String>,
    /// yt-dlp binary for downloading from Youtube
    #[arg(long)]
    #[cfg_attr(windows, arg(default_value = Some("./bin/yt-dlp.exe")))]
    #[cfg_attr(unix, arg(default_value = Some("./bin/yt-dlp")))]
    ytdlp_binary_path: Option<String>,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "INFO");
    }
    env_logger::init();

    let total_transcode_threads: usize = match args.total_transcode_threads {
        0 => std::thread::available_parallelism().map(|v| v.get()).unwrap_or(1),
        x => x,
    };
    let total_worker_threads: usize = match args.total_worker_threads {
        0 => std::thread::available_parallelism().map(|v| v.get()).unwrap_or(1),
        x => x,
    };
    let mut app_config = AppConfig::default();
    if let Some(path) = args.ytdlp_binary_path { app_config.ytdlp_binary = PathBuf::from(path); }
    if let Some(path) = args.ffmpeg_binary_path { app_config.ffmpeg_binary = PathBuf::from(path); }
    app_config.seed_directories()?;
    let app_state = AppState::new(app_config, total_transcode_threads)?;
    // start server
    const API_PREFIX: &str = "/api/v1";
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(web::scope(API_PREFIX)
                .service(routes::request_transcode)
                .service(routes::delete_transcode)
                .service(routes::delete_download)
                .service(routes::get_downloads)
                .service(routes::get_transcodes)
                .service(routes::get_download)
                .service(routes::get_transcode)
                .service(routes::get_download_state)
                .service(routes::get_transcode_state)
                .service(routes::get_download_link)
                .service(routes::get_metadata)
            )
            .service(actix_files::Files::new("/data", "./data/").show_files_listing())
            .service(actix_files::Files::new("/", "./static/").index_file("index.html"))
            // NOTE: There is little benefit to using compress middleware when serving audio files
            // since they are already extremely compressed. Additionally it also ends up removing
            // the Content-Length header from the downloads since the file is being streamed.
            // This has the effect of removing any progress bar on the download which is a bad experience.
            // .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
    })
    .bind((args.url, args.port))?
    .workers(total_worker_threads)
    .run()
    .await?;
    Ok(())
}
