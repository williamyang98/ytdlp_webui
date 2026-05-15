use anyhow::Context;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use dashmap::DashMap;
use crate::{
    database::{Database, VideoId},
    youtube_metadata::{YoutubeMetadataCache, YoutubeMetadata, get_youtube_metadata},
    workers::{WorkerCacheEntry, WorkerThreadPool},
    worker_download::{DownloadCache, DownloadState},
    worker_transcode::{TranscodeCache, TranscodeKey, TranscodeState},
};


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
pub struct App {
    pub app_config: Arc<AppConfig>,
    pub database: Arc<Database>,
    pub worker_thread_pool: WorkerThreadPool,
    pub download_cache: DownloadCache,
    pub transcode_cache: TranscodeCache,
    pub metadata_cache: YoutubeMetadataCache,
}

impl App {
    pub fn new(app_config: AppConfig) -> anyhow::Result<Self> {
        let database = Database::open(app_config.database_path.to_string_lossy().as_ref())?;
        let database = Arc::new(database);
        database.connect()?.run_pending_migrations();

        let worker_thread_pool: WorkerThreadPool = Arc::new(Mutex::new(ThreadPool::new(app_config.total_transcode_threads)));
        let download_cache: DownloadCache = Arc::new(DashMap::<VideoId, WorkerCacheEntry<DownloadState>>::new());
        let transcode_cache: TranscodeCache = Arc::new(DashMap::<TranscodeKey, WorkerCacheEntry<TranscodeState>>::new());
        let metadata_cache: YoutubeMetadataCache = Arc::new(DashMap::<VideoId, Arc<YoutubeMetadata>>::new());
        Ok(Self {
            app_config: Arc::new(app_config),
            database,
            worker_thread_pool,
            download_cache,
            transcode_cache,
            metadata_cache,
        })
    }

    pub async fn get_youtube_metadata_from_cache(&self, video_id: VideoId) -> anyhow::Result<Arc<YoutubeMetadata>> {
        if let Some(metadata) = self.metadata_cache.get(&video_id) {
            return Ok(metadata.clone());
        }
        let metadata = get_youtube_metadata(&video_id).await?;
        let metadata = Arc::new(metadata);
        self.metadata_cache.insert(video_id, metadata.clone());
        Ok(metadata)
    }
}
