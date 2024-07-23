use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
use dashmap::DashMap;
use threadpool::ThreadPool;
use ytdlp_server::{
    app::{AppConfig, WorkerThreadPool, WorkerCacheEntry},
    database::{DatabasePool, VideoId, setup_database},
    worker_download::{DownloadCache, DownloadState},
    worker_transcode::{TranscodeCache, TranscodeKey, TranscodeState},
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
    ffmpeg_binary_path: Option<String>,
    /// yt-dlp binary for downloading from Youtube
    #[arg(long)]
    ytdlp_binary_path: Option<String>,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
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
    let db_manager = r2d2_sqlite::SqliteConnectionManager::file(app_config.root.join("index.db"));
    let db_pool = DatabasePool::new(db_manager)?;
    setup_database(db_pool.get()?)?;
    let download_cache: DownloadCache = Arc::new(DashMap::<VideoId, WorkerCacheEntry<DownloadState>>::new());
    let transcode_cache: TranscodeCache = Arc::new(DashMap::<TranscodeKey, WorkerCacheEntry<TranscodeState>>::new());
    let worker_thread_pool: WorkerThreadPool = Arc::new(Mutex::new(ThreadPool::new(total_transcode_threads)));
    // start server
    const API_PREFIX: &str = "/api/v1";
    HttpServer::new(move || {
        App::new()
            .app_data(app_config.clone())
            .app_data(db_pool.clone())
            .app_data(worker_thread_pool.clone())
            .app_data(download_cache.clone())
            .app_data(transcode_cache.clone())
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
            )
            .service(actix_files::Files::new("/data", "./data/").show_files_listing())
            .service(actix_files::Files::new("/", "./static/").index_file("index.html"))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
    })
    .bind((args.url, args.port))?
    .workers(total_worker_threads)
    .run()
    .await?;
    Ok(())
}
