use std::sync::{Arc, Mutex};
use actix_web::{web, App, HttpServer};
use clap::Parser;
use dashmap::DashMap;
use threadpool::ThreadPool;
use ytdlp_server::{
    app::{AppConfig, WorkerThreadPool},
    database::{DatabasePool, VideoId, setup_database},
    worker_download::{DownloadCache, DownloadState},
    worker_transcode::{TranscodeCache, TranscodeKey, TranscodeState},
    routes,
};



#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Maximum number of worker threads
    #[arg(short, long, default_value_t = 0)]
    total_worker_threads: usize,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut env_logger_builder = env_logger::Builder::new();
    env_logger_builder.filter_level(log::LevelFilter::Debug).init();

    let total_worker_threads: usize = match args.total_worker_threads {
        0 => std::thread::available_parallelism().map(|v| v.get()*4).unwrap_or(1),
        x => x,
    };
    let app_config = AppConfig::default();
    app_config.seed_directories()?;
    let db_manager = r2d2_sqlite::SqliteConnectionManager::file(app_config.root.join("index.db"));
    let db_pool = DatabasePool::new(db_manager)?;
    setup_database(db_pool.get()?)?;
    let download_cache: DownloadCache = Arc::new(DashMap::<VideoId, DownloadState>::new());
    let transcode_cache: TranscodeCache = Arc::new(DashMap::<TranscodeKey, TranscodeState>::new());
    let worker_thread_pool: WorkerThreadPool = Arc::new(Mutex::new(ThreadPool::new(total_worker_threads)));
    // start server
    const API_PREFIX: &str = "/api/v1";
    HttpServer::new(move || {
        App::new()
            .app_data(app_config.clone())
            .app_data(db_pool.clone())
            .app_data(worker_thread_pool.clone())
            .app_data(download_cache.clone())
            .app_data(transcode_cache.clone())
            .service(routes::index)
            .service(web::scope(API_PREFIX).service(routes::request_audio))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;
    Ok(())
}
