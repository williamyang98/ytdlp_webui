use crate::app_config::AppConfig;
use crate::database::{Database, FfmpegRow, TranscodeKey, VideoId, YtdlpRow};
use crate::util::defer;
use crate::worker_download::{DownloadWorker, DownloadWorkers};
use crate::worker_transcode::{TranscodeWorker, TranscodeWorkers};
use crate::youtube_metadata::{YoutubeMetadata, YoutubeMetadataCache, get_youtube_metadata};
use dashmap::DashMap;
use std::{path::PathBuf, sync::Arc};
use threadpool::ThreadPool;
use serde::Serialize;

#[derive(Clone)]
pub struct App {
    app_config: Arc<AppConfig>,
    database: Arc<Database>,
    threadpool: Arc<ThreadPool>,
    transcode_workers: Arc<TranscodeWorkers>,
    download_workers: Arc<DownloadWorkers>,
    metadata_cache: YoutubeMetadataCache,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum DeleteFileResult {
    Success { filename: String },
    Failure { filename: String, reason: String },
}

#[derive(Debug,Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum DeleteResponse {
    Busy,
    Success { paths: Vec<DeleteFileResult> },
}

impl App {
    pub fn new(app_config: AppConfig) -> anyhow::Result<Self> {
        let app_config = Arc::new(app_config);
        let database = Database::open(app_config.database_path.to_string_lossy().as_ref())?;
        let database = Arc::new(database);
        database.connect()?.run_pending_migrations();

        let threadpool = Arc::new(ThreadPool::new(app_config.total_transcode_threads));
        let download_workers = Arc::new(DownloadWorkers::new(database.clone(), threadpool.clone(), app_config.clone()));
        let transcode_workers = Arc::new(TranscodeWorkers::new(database.clone(), threadpool.clone(), app_config.clone(), download_workers.clone()));
        let metadata_cache: YoutubeMetadataCache = Arc::new(DashMap::<VideoId, Arc<YoutubeMetadata>>::new());
        Ok(Self {
            app_config,
            database,
            threadpool,
            download_workers,
            transcode_workers,
            metadata_cache,
        })
    }

    pub async fn get_youtube_metadata_from_cache(&self, video_id: &VideoId) -> anyhow::Result<Arc<YoutubeMetadata>> {
        if let Some(metadata) = self.metadata_cache.get(video_id) {
            return Ok(metadata.clone());
        }
        let metadata = get_youtube_metadata(video_id).await?;
        let metadata = Arc::new(metadata);
        self.metadata_cache.insert(video_id.clone(), metadata.clone());
        Ok(metadata)
    }

    pub async fn start_transcode(&self, key: &TranscodeKey) -> anyhow::Result<Arc<TranscodeWorker>> {
        let metadata = self.get_youtube_metadata_from_cache(&key.video_id).await?;
        self.transcode_workers.start_worker(key, Some(metadata))
    }

    pub fn start_download(&self, video_id: &VideoId) -> anyhow::Result<Arc<DownloadWorker>> {
        self.download_workers.start_worker(video_id)
    }

    pub fn delete_download(&self, video_id: &VideoId) -> anyhow::Result<Option<DeleteResponse>> {
        if let Some(worker) = self.download_workers.get_worker(video_id) {
            if worker.get_status().is_busy() {
                return Ok(Some(DeleteResponse::Busy));
            }
        }
        defer(|| {
            self.download_workers.delete_worker(video_id);
        });

        let mut db_conn =self.database.connect()?;
        let Some(entry) = db_conn.select_ytdlp_entry(video_id)? else {
            return Ok(None);
        };
        let total_deleted = db_conn.delete_ytdlp_entry(video_id)?;
        if total_deleted != 1 {
            log::warn!("Failed to delete ytdlp entry: {0}", video_id.as_str());
        }
        drop(db_conn);

        let paths = &[entry.audio_path, entry.stdout_log_path, entry.stderr_log_path, entry.system_log_path];
        let deleted_files: Vec<DeleteFileResult> = paths
            .iter()
            .flatten()
            .filter_map(|path| {
                let abs_path = PathBuf::from(path);
                let Ok(abs_path) = self.app_config.get_absolute_data_filepath(&abs_path) else {
                    return None;
                };
                match std::fs::remove_file(abs_path) {
                    Ok(()) => Some(DeleteFileResult::Success { filename: path.clone() }),
                    Err(err) => Some(DeleteFileResult::Failure { filename: path.clone(), reason: err.to_string() }),
                }
            })
            .collect();
        Ok(Some(DeleteResponse::Success { paths: deleted_files }))
    }

    pub fn delete_transcode(&self, key: &TranscodeKey) -> anyhow::Result<Option<DeleteResponse>> {
        if let Some(worker) = self.transcode_workers.get_worker(key) {
            if worker.get_status().is_busy() {
                return Ok(Some(DeleteResponse::Busy));
            }
        }
        defer(|| {
            self.transcode_workers.delete_worker(key);
        });

        let mut db_conn =self.database.connect()?;
        let Some(entry) = db_conn.select_ffmpeg_entry(key)? else {
            return Ok(None);
        };
        let total_deleted = db_conn.delete_ffmpeg_entry(key)?;
        if total_deleted != 1 {
            log::warn!("Failed to delete ffmpeg entry: {0}", key.as_str());
        }
        drop(db_conn);

        let paths = &[entry.audio_path, entry.stdout_log_path, entry.stderr_log_path, entry.system_log_path];
        let deleted_files: Vec<DeleteFileResult> = paths
            .iter()
            .flatten()
            .filter_map(|path| {
                let abs_path = PathBuf::from(path);
                let Ok(abs_path) = self.app_config.get_absolute_data_filepath(&abs_path) else {
                    return None;
                };
                match std::fs::remove_file(abs_path) {
                    Ok(()) => Some(DeleteFileResult::Success { filename: path.clone() }),
                    Err(err) => Some(DeleteFileResult::Failure { filename: path.clone(), reason: err.to_string() }),
                }
            })
            .collect();
        Ok(Some(DeleteResponse::Success { paths: deleted_files }))
    }

    pub fn get_downloads(&self) -> anyhow::Result<Vec<YtdlpRow>> {
        let entries = self.database
            .connect()?
            .select_ytdlp_entries()?;
        Ok(entries)
    }

    pub fn get_transcodes(&self) -> anyhow::Result<Vec<FfmpegRow>> {
        let entries = self.database
            .connect()?
            .select_ffmpeg_entries()?;
        Ok(entries)
    }

    pub fn get_download(&self, video_id: &VideoId) -> anyhow::Result<Option<YtdlpRow>> {
        let entry = self.database
            .connect()?
            .select_ytdlp_entry(video_id)?;
        Ok(entry)
    }

    pub fn get_transcode(&self, key: &TranscodeKey) -> anyhow::Result<Option<FfmpegRow>> {
        let entry = self.database
            .connect()?
            .select_ffmpeg_entry(key)?;
        Ok(entry)
    }

    pub fn get_download_worker(&self, video_id: &VideoId) -> Option<Arc<DownloadWorker>> {
        self.download_workers.get_worker(video_id)
    }

    pub fn get_transcode_worker(&self, key: &TranscodeKey) -> Option<Arc<TranscodeWorker>> {
        self.transcode_workers.get_worker(key)
    }

    pub fn get_download_abspath(&self, key: &TranscodeKey) -> anyhow::Result<Option<PathBuf>> {
        let entry = self.get_transcode(key)?;
        let Some(entry) = entry else {
            return Ok(None);
        };
        let Some(audio_path) = entry.audio_path else {
            return Ok(None);
        };
        let path = self.app_config.get_absolute_data_filepath(&PathBuf::from(audio_path))?;
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(path))
    }
}
