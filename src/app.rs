use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Condvar};
use thiserror::Error;
use threadpool::ThreadPool;
use dashmap::DashMap;
use crate::{
    database::{DatabasePool, VideoId, setup_database},
    metadata::{MetadataCache, Metadata},
    worker_download::{DownloadCache, DownloadState},
    worker_transcode::{TranscodeCache, TranscodeKey, TranscodeState},
};

pub type WorkerThreadPool = Arc<Mutex<ThreadPool>>;
pub type WorkerCacheEntry<T> = Arc<(Mutex<T>, Condvar)>;

#[derive(Debug,Error)]
pub enum WorkerError {
    #[error("Failed to create stdout log: {0:?}")]
    StdoutLogCreate(std::io::Error),
    #[error("Failed to create stderr log: {0:?}")]
    StderrLogCreate(std::io::Error),
    #[error("Failed to write to system log: {0:?}")]
    SystemWriteFail(std::io::Error),
    #[error("Failed to write to stdout log: {0:?}")]
    StdoutWriteFail(std::io::Error),
    #[error("Failed to write to stderr log: {0:?}")]
    StderrWriteFail(std::io::Error),
    #[error("Failed to acquire stdout from process")]
    StdoutMissing,
    #[error("Failed to acquire stderr from process")]
    StderrMissing,
    #[error("Failed to join stdout thread: {0:?}")]
    StdoutThreadJoin(Box<dyn std::any::Any + Send + 'static>),
    #[error("Failed to join stderr thread: {0:?}")]
    StderrThreadJoin(Box<dyn std::any::Any + Send + 'static>),
}

#[derive(Clone,Debug)]
pub struct AppConfig {
    pub root: PathBuf,
    pub data: PathBuf,
    pub download: PathBuf,
    pub transcode: PathBuf,
    pub ffmpeg_binary: PathBuf,
    pub ytdlp_binary: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let root = Path::new(".");
        let data = root.join("data");
        Self {
            root: root.to_owned(),
            data: data.to_owned(), 
            download: data.join("downloads"),
            transcode: data.join("transcode"),
            ffmpeg_binary: root.join("bin").join("ffmpeg.exe"),
            ytdlp_binary: root.join("bin").join("yt-dlp.exe"),
        }
    }
}

impl AppConfig {
    pub fn seed_directories(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.data)?;
        std::fs::create_dir_all(&self.download)?;
        std::fs::create_dir_all(&self.transcode)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct AppState {
    pub app_config: Arc<AppConfig>,
    pub db_pool: DatabasePool,
    pub worker_thread_pool: WorkerThreadPool,
    pub download_cache: DownloadCache,
    pub transcode_cache: TranscodeCache,
    pub metadata_cache: MetadataCache,
}

impl AppState {
    pub fn new(app_config: AppConfig, total_transcode_threads: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let db_manager = r2d2_sqlite::SqliteConnectionManager::file(app_config.data.join("index.db"));
        let db_pool = DatabasePool::new(db_manager)?;
        setup_database(db_pool.get()?)?;
        let worker_thread_pool: WorkerThreadPool = Arc::new(Mutex::new(ThreadPool::new(total_transcode_threads)));
        let download_cache: DownloadCache = Arc::new(DashMap::<VideoId, WorkerCacheEntry<DownloadState>>::new());
        let transcode_cache: TranscodeCache = Arc::new(DashMap::<TranscodeKey, WorkerCacheEntry<TranscodeState>>::new());
        let metadata_cache: MetadataCache = Arc::new(DashMap::<VideoId, Arc<Metadata>>::new());
        Ok(Self {
            app_config: Arc::new(app_config),
            db_pool, 
            worker_thread_pool,
            download_cache,
            transcode_cache,
            metadata_cache,
        })
    }
}
