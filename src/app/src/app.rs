use anyhow::Context;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Condvar};
use thiserror::Error;
use threadpool::ThreadPool;
use dashmap::DashMap;
use crate::{
    database::{DatabasePool, VideoId, open_database, create_database},
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
    pub current_working_directory: PathBuf,
    pub data_folder: PathBuf,
    pub static_folder: PathBuf,
    pub downloads_folder: PathBuf,
    pub transcodes_folder: PathBuf,
    pub binaries_folder: PathBuf,
    pub database_path: PathBuf,
    pub total_transcode_threads: usize,
}

impl AppConfig {
    pub fn new(data_folder: &Path, static_folder: &Path) -> anyhow::Result<Self> {
        let downloads_folder = data_folder.join("downloads");
        let transcodes_folder = data_folder.join("transcodes");
        let binaries_folder = data_folder.join("binaries");
        let database_path = data_folder.join("index.db");
        let current_working_directory = std::env::current_dir()
            .context("Couldn't get current working directory of process")?;

        std::fs::create_dir_all(data_folder).context("Couldn't create data folder")?;
        std::fs::create_dir_all(&downloads_folder).context("Couldn't create downloads folder")?;
        std::fs::create_dir_all(&transcodes_folder).context("Couldn't create transcodes folder")?;
        std::fs::create_dir_all(&binaries_folder).context("Couldn't create binaries folder")?;
        if !static_folder.exists() {
            return Err(anyhow::anyhow!("Static folder doesn't exist"));
        }
        if !static_folder.is_dir() {
            return Err(anyhow::anyhow!("Static folder isn't a directory"));
        }

        Ok(Self {
            current_working_directory,
            data_folder: data_folder.to_path_buf(),
            static_folder: static_folder.to_path_buf(),
            downloads_folder,
            transcodes_folder,
            binaries_folder,
            database_path,
            total_transcode_threads: 8,
        })
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
    pub fn new(app_config: AppConfig) -> anyhow::Result<Self> {
        let db_pool = open_database(app_config.database_path.to_string_lossy().as_ref())?;
        {
            let mut db_conn = db_pool.get()?;
            create_database(&mut db_conn);
        }
        let worker_thread_pool: WorkerThreadPool = Arc::new(Mutex::new(ThreadPool::new(app_config.total_transcode_threads)));
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
