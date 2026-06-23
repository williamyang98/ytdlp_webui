use crate::app::AppThreadPool;
use crate::app_config::AppConfig;
use crate::database::{Database, WorkerStatus};
use crate::util::get_unix_time;
use crate::process::Process;
use crate::ytdlp;
use anyhow::Context;
use derive_more::Debug;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use uuid::Uuid;
use youtube_api::VideoId;

#[derive(Clone,Debug,Serialize)]
pub struct DownloadState {
    pub id: Uuid,
    pub worker_status: WorkerStatus,
    pub file_cached: bool,
    pub fail_reason: Option<String>,
    pub start_time_unix: u64,
    pub end_time_unix: u64,
    pub eta_seconds: Option<u64>,
    pub elapsed_seconds: Option<u64>,
    pub downloaded_bytes: Option<usize>,
    pub total_bytes: Option<usize>,
    pub speed_bytes: Option<usize>,
}

impl Default for DownloadState {
    fn default() -> Self {
        let curr_time = get_unix_time();
        Self {
            id: Uuid::new_v4(),
            worker_status: WorkerStatus::Queued,
            file_cached: false,
            fail_reason: None,
            start_time_unix: curr_time,
            end_time_unix: curr_time,
            eta_seconds: None,
            elapsed_seconds: None,
            downloaded_bytes: None,
            total_bytes: None,
            speed_bytes: None,
        }
    }
}

fn update_field<T>(dst: &mut Option<T>, src: Option<T>) {
    if src.is_some() {
        *dst = src;
    }
}

impl DownloadState {
    pub fn update_from_ytdlp(&mut self, progress: &ytdlp::DownloadProgress) {
        self.end_time_unix = get_unix_time();
        update_field(&mut self.eta_seconds, progress.eta_seconds);
        update_field(&mut self.elapsed_seconds, progress.elapsed_seconds);
        update_field(&mut self.downloaded_bytes, progress.downloaded_bytes);
        update_field(&mut self.total_bytes, progress.total_bytes);
        update_field(&mut self.speed_bytes, progress.speed_bytes);
    }
}

pub struct DownloadWorker {
    video_id: VideoId,
    state: Mutex<Box<DownloadState>>,
    condvar: Condvar,
}

impl DownloadWorker {
    fn new(video_id: VideoId) -> Self {
        Self {
            video_id,
            state: Mutex::new(Box::new(DownloadState::default())),
            condvar: Condvar::new(),
        }
    }

    pub fn get_status(&self) -> WorkerStatus {
        self.state.lock().unwrap().worker_status
    }

    pub fn wait_busy(&self) {
        let mut state = self.state.lock().unwrap();
        loop {
            if !state.worker_status.is_busy() {
                break;
            }
            state = self.condvar.wait(state).unwrap();
        }
    }

    pub fn get_state(&self) -> Box<DownloadState> {
        self.state.lock().unwrap().clone()
    }
}

pub struct DownloadWorkers {
    database: Arc<Database>,
    threadpool: Arc<AppThreadPool>,
    app_config: Arc<AppConfig>,
    ytdlp: Arc<RwLock<ytdlp::Ytdlp>>,
    cache: Mutex<HashMap<VideoId, Arc<DownloadWorker>>>,
}

impl DownloadWorkers {
    pub fn new(database: Arc<Database>, threadpool: Arc<AppThreadPool>, app_config: Arc<AppConfig>, ytdlp: Arc<RwLock<ytdlp::Ytdlp>>) -> Self {
        Self {
            database,
            threadpool,
            app_config,
            ytdlp,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_worker(&self, video_id: &VideoId) -> Option<Arc<DownloadWorker>> {
        let cache = self.cache.lock().unwrap();
        cache.get(video_id).cloned()
    }

    pub fn delete_worker(&self, video_id: &VideoId) -> Option<Arc<DownloadWorker>> {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(video_id)
    }

    pub fn start_worker(&self, video_id: &VideoId) -> anyhow::Result<Arc<DownloadWorker>> {
        // check cache hit
        let mut cache = self.cache.lock().unwrap();
        if let Some(worker) = cache.get(video_id) {
            let state = worker.state.lock().unwrap();
            if state.worker_status.is_healthy() {
                return Ok(worker.clone());
            }
        }
        // new cache item
        let worker = Arc::new(DownloadWorker::new(video_id.clone()));
        let _old_worker = cache.insert(video_id.clone(), worker.clone());
        drop(cache);
        // check database item
        if let Some(db_entry) = self.database.connect()?.select_ytdlp_entry(video_id)? {
            if db_entry.status == WorkerStatus::Finished {
                if let Some(audio_path) = db_entry.audio_path {
                    let audio_path = self.app_config.data_folder.join(audio_path);
                    if audio_path.is_file() {
                        let mut state = worker.state.lock().unwrap();
                        state.worker_status = WorkerStatus::Finished;
                        state.file_cached = true;
                        state.start_time_unix = db_entry.unix_time.as_u64();
                        state.end_time_unix = db_entry.unix_time.as_u64();
                        worker.condvar.notify_all();
                        drop(state);
                        return Ok(worker);
                    } else {
                        log::warn!("Restarting download because audio file was missing: {0}", audio_path.to_string_lossy());
                    }
                }
            }
        }
        // new database item
        {
            let mut db_conn = self.database.connect()?;
            db_conn.delete_ytdlp_entry(video_id)?;
            db_conn.insert_ytdlp_entry(video_id)?;
            db_conn.select_and_update_ytdlp_entry(video_id, |entry| {
                entry.status = WorkerStatus::Queued;
                entry.unix_time = get_unix_time().into();
            })?;
        }
        let inner_runner = {
            let video_id = video_id.clone();
            let database = self.database.clone();
            let app_config = self.app_config.clone();
            let ytdlp = self.ytdlp.clone();
            let worker = worker.clone();
            move || -> anyhow::Result<()> {
                let Ok(ytdlp) = ytdlp.try_read() else {
                    return Err(anyhow::anyhow!("Ytdlp is busy updating"));
                };
                // setup process
                let output_dirpath = app_config.downloads_folder.join(video_id.as_str());
                let output_dirpath = std::path::absolute(&output_dirpath)
                    .with_context(|| format!("Failed to get absolute output filepath from: {0}", output_dirpath.to_string_lossy()))?;
                std::fs::create_dir_all(&output_dirpath)
                    .with_context(|| format!("Failed to create folder for download worker: {0}", output_dirpath.to_string_lossy()))?;
                let command = ytdlp.create_download_command(&video_id, &output_dirpath, &app_config.current_working_directory, &app_config.ffmpeg_command);
                let mut process = Process::new(command);
                process.label = Some(format!("download_{0}", video_id.as_str()));
                process.system_log_filename = Some(output_dirpath.join("system.log"));
                process.stdout_filename = Some(output_dirpath.join("stderr.log"));
                process.stderr_filename = Some(output_dirpath.join("stdout.log"));

                let download_filepath = Arc::new(Mutex::new(None));
                process.stdout_handler = Some(Box::new({
                    let download_filepath = download_filepath.clone();
                    let worker = worker.clone();
                    move |line: Option<&str>| {
                        let Some(line) = line else {
                            return;
                        };
                        match ytdlp::parse_stdout_line(line) {
                            None => (),
                            Some(ytdlp::ParsedStdoutLine::DownloadProgress(ref progress)) => {
                                log::debug!("[download] id={0} progress={1:?}", worker.video_id.as_str(), progress);
                                worker.state.lock().unwrap().update_from_ytdlp(progress);
                            },
                            Some(ytdlp::ParsedStdoutLine::OutputPath(path)) => {
                                *download_filepath.lock().unwrap() = Some(path);
                            },
                        }
                    }
                }));

                let extract_filepath = Arc::new(Mutex::new(None));
                process.stderr_handler = Some(Box::new({
                    let extract_filepath = extract_filepath.clone();
                    move |line: Option<&str>| {
                        let Some(line) = line else {
                            return;
                        };
                        match ytdlp::parse_stderr_line(line) {
                            None => (),
                            Some(ytdlp::ParsedStderrLine::UsageError(message)) => {
                                *extract_filepath.lock().unwrap() = Some(Err(anyhow::anyhow!("Usage error: {message}")));
                            },
                            Some(ytdlp::ParsedStderrLine::ExtractPath(path)) => {
                                *extract_filepath.lock().unwrap() = Some(Ok(path));
                            },
                            Some(ytdlp::ParsedStderrLine::VideoUnavailableError { video_id, message }) => {
                                *extract_filepath.lock().unwrap() = Some(Err(anyhow::anyhow!("Video {video_id} unavailable: {message}")));
                            },
                            Some(ytdlp::ParsedStderrLine::AgeRestrictedError { video_id, message: _message }) => {
                                *extract_filepath.lock().unwrap() = Some(Err(anyhow::anyhow!("Video {video_id} age restricted")));
                            },
                            Some(ytdlp::ParsedStderrLine::UnhandledError { video_id, message }) => {
                                *extract_filepath.lock().unwrap() = Some(Err(anyhow::anyhow!("Error downloading video {video_id}: {message}")));
                            },
                        }
                    }
                }));
                // update logging files
                let as_relative_filepath = |path: Option<&PathBuf>| -> Option<String> {
                    let path = path.as_ref()?;
                    let Ok(path) = app_config.get_relative_data_path(path) else {
                        return None;
                    };
                    Some(path.to_string_lossy().to_string())
                };
                database
                    .connect()?
                    .select_and_update_ytdlp_entry(&video_id, {
                        let system_log_path = as_relative_filepath(process.system_log_filename.as_ref());
                        let stdout_log_path = as_relative_filepath(process.stdout_filename.as_ref());
                        let stderr_log_path = as_relative_filepath(process.stderr_filename.as_ref());
                        move |entry| {
                            entry.system_log_path = system_log_path;
                            entry.stdout_log_path = stdout_log_path;
                            entry.stderr_log_path = stderr_log_path;
                        }
                    })?;
                if let Err(err) = process.run() {
                    return Err(anyhow::anyhow!("Failed to run process: {err:?}"));
                }
                // NOTE: Audio extractor for yt-dlp might not extract anything if the file extension remains the same
                let download_filepath: Option<String> = download_filepath.lock().unwrap().clone();
                let extract_filepath = extract_filepath.lock().unwrap()
                    .take()
                    .transpose()?;
                if let Some(filepath) = &download_filepath {
                    log::debug!("Got ytdlp download filepath: {0}", filepath);
                }
                if let Some(filepath) = &extract_filepath {
                    log::debug!("Got ytdlp extract filepath: {0}", &filepath);
                }
                let output_filepath = extract_filepath.or(download_filepath);
                let Some(output_filepath) = output_filepath else {
                    return Err(anyhow::anyhow!("Failed to get output filepath"));
                };
                let output_filepath = PathBuf::from(output_filepath);
                if !output_filepath.is_file() {
                    return Err(anyhow::anyhow!("Output filepath doesn't exist: {0}", output_filepath.to_string_lossy()));
                }
                log::debug!("Got ytdlp output filepath: {0}", output_filepath.to_string_lossy());
                database
                    .connect()?
                    .select_and_update_ytdlp_entry(&video_id, move |entry| {
                        entry.audio_path = as_relative_filepath(Some(&output_filepath));
                    })?;
                Ok(())
            }
        };

        let outer_runner = {
            let video_id = video_id.clone();
            let worker = worker.clone();
            let database = self.database.clone();
            move || -> anyhow::Result<()> {
                {
                    database
                        .connect()?
                        .select_and_update_ytdlp_entry(&video_id, move |entry| {
                            entry.status = WorkerStatus::Running;
                        })?;
                    let mut state = worker.state.lock().unwrap();
                    state.worker_status = WorkerStatus::Running;
                    worker.condvar.notify_all();
                }
                match inner_runner() {
                    Err(err) => {
                        database
                            .connect()?
                            .select_and_update_ytdlp_entry(&video_id, move |entry| {
                                entry.status = WorkerStatus::Failed;
                            })?;
                        let mut state = worker.state.lock().unwrap();
                        state.worker_status = WorkerStatus::Failed;
                        state.fail_reason = Some(err.to_string());
                        state.end_time_unix = get_unix_time();
                        worker.condvar.notify_all();
                    },
                    Ok(()) => {
                        database
                            .connect()?
                            .select_and_update_ytdlp_entry(&video_id, move |entry| {
                                entry.status = WorkerStatus::Finished;
                            })?;
                        let mut state = worker.state.lock().unwrap();
                        state.worker_status = WorkerStatus::Finished;
                        worker.condvar.notify_all();
                    },
                }
                Ok(())
            }
        };

        self.threadpool.downloads_worker.execute(move || {
            if let Err(err) = outer_runner() {
                log::error!("Runner failed with: {0}", err);
            }
        });
        Ok(worker)
    }
}
