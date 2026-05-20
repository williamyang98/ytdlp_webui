use crate::app_config::AppConfig;
use crate::database::{Database, FfmpegRow, TranscodeKey, YtdlpRow};
use crate::util::defer;
use crate::worker_download::{DownloadWorker, DownloadWorkers};
use crate::worker_transcode::{TranscodeWorker, TranscodeWorkers};
use crate::youtube_api_cache::YoutubeApiCache;
use youtube_api::{PaginatedResponse, PlaylistId, PlaylistItem, VideoId, VideoItem};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use threadpool::ThreadPool;


#[derive(Clone)]
pub struct App {
    app_config: Arc<AppConfig>,
    database: Arc<Database>,
    _threadpool: Arc<ThreadPool>,
    transcode_workers: Arc<TranscodeWorkers>,
    download_workers: Arc<DownloadWorkers>,
    youtube_api_cache: Arc<YoutubeApiCache>
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum DeleteFileResult {
    Success { filename: PathBuf },
    Failure { filename: PathBuf, reason: String },
}

#[derive(Debug)]
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
        let youtube_api_cache = Arc::new(YoutubeApiCache::new(app_config.clone())?);
        Ok(Self {
            app_config,
            database,
            _threadpool: threadpool,
            download_workers,
            transcode_workers,
            youtube_api_cache,
        })
    }


    pub fn start_transcode(&self, key: &TranscodeKey, video_info: Option<Arc<PaginatedResponse<VideoItem>>>) -> anyhow::Result<Arc<TranscodeWorker>> {
        self.transcode_workers.start_worker(key, video_info)
    }

    pub fn start_download(&self, video_id: &VideoId) -> anyhow::Result<Arc<DownloadWorker>> {
        self.download_workers.start_worker(video_id)
    }

    fn delete_files<I, P>(&self, relative_paths: I) -> Vec<DeleteFileResult>
    where
        I: IntoIterator<Item = P>,
        PathBuf: From<P>,
    {
        let mut deleted_results = vec![];
        let mut relative_dirs = vec![];
        // delete files
        for relative_path in relative_paths {
            let relative_path = PathBuf::from(relative_path);
            match self.app_config.delete_file(&relative_path) {
                Ok(()) => {
                    if let Some(relative_dir) = relative_path.parent() {
                        relative_dirs.push(relative_dir.to_path_buf());
                    }
                    deleted_results.push(DeleteFileResult::Success { filename: relative_path.clone() });
                },
                Err(err) => {
                    deleted_results.push(DeleteFileResult::Failure { filename: relative_path.clone(), reason: err.to_string() });
                },
            };
        }
        // delete folders
        relative_dirs.sort_unstable();
        relative_dirs.dedup();
        for relative_dir in &relative_dirs {
            match self.app_config.delete_folder(relative_dir) {
                Ok(()) => {
                    deleted_results.push(DeleteFileResult::Success { filename: relative_dir.clone() });
                },
                Err(err) => {
                    deleted_results.push(DeleteFileResult::Failure { filename: relative_dir.clone(), reason: err.to_string() });
                },
            }
        }
        deleted_results
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
        let paths = paths
            .iter()
            .flatten()
            .map(PathBuf::from);
        let paths = self.delete_files(paths);

        Ok(Some(DeleteResponse::Success { paths }))
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
        let paths = paths
            .iter()
            .flatten()
            .map(PathBuf::from);
        let paths = self.delete_files(paths);

        Ok(Some(DeleteResponse::Success { paths }))
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

    pub async fn get_youtube_video(&self, video_id: &VideoId) -> anyhow::Result<Arc<PaginatedResponse<VideoItem>>> {
        self.youtube_api_cache.get_video(video_id).await
    }

    pub async fn get_youtube_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Arc<PaginatedResponse<PlaylistItem>>> {
        self.youtube_api_cache.get_playlist(playlist_id).await
    }

    pub fn get_download_abspath(&self, key: &TranscodeKey) -> anyhow::Result<Option<PathBuf>> {
        let entry = self.get_transcode(key)?;
        let Some(entry) = entry else {
            return Ok(None);
        };
        let Some(audio_path) = entry.audio_path else {
            return Ok(None);
        };
        let path = self.app_config.get_absolute_data_path(&PathBuf::from(audio_path))?;
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(path))
    }
}
