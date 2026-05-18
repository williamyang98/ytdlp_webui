use crate::app_config::AppConfig;
use crate::database::{Database, VideoId, WorkerStatus};
use crate::util::get_unix_time;
use crate::worker_process::{ProcessPipeHandler, ProcessWorker};
use crate::ytdlp;
use anyhow::Context;
use dashmap::DashMap;
use derive_more::Debug;
use serde::Serialize;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use threadpool::ThreadPool;

#[derive(Clone,Debug,Serialize)]
pub struct DownloadState {
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

#[derive(Clone)]
struct YtdlpStdoutHandler {
    worker: Arc<DownloadWorker>,
    download_path: Arc<Mutex<Option<String>>>,
}

impl YtdlpStdoutHandler {
    pub fn new(worker: Arc<DownloadWorker>) -> Self {
        Self {
            worker,
            download_path: Arc::new(Mutex::new(None)),
        }
    }
}

impl ProcessPipeHandler for YtdlpStdoutHandler {
    fn read_line(&self, line: &str) -> ControlFlow<()> {
        match ytdlp::parse_stdout_line(line) {
            None => (),
            Some(ytdlp::ParsedStdoutLine::DownloadProgress(ref progress)) => {
                log::debug!("[download] id={0} progress={1:?}", self.worker.video_id.as_str(), progress);
                self.worker.state.lock().unwrap().update_from_ytdlp(progress);
            },
            Some(ytdlp::ParsedStdoutLine::OutputPath(path)) => {
                *self.download_path.lock().unwrap() = Some(path);
            },
        }
        ControlFlow::Continue(())
    }

    fn finish(&self) {

    }
}

#[derive(Clone)]
struct YtdlpStderrHandler {
    extract_path: Arc<Mutex<Option<anyhow::Result<String>>>>,
}

impl Default for YtdlpStderrHandler {
    fn default() -> Self {
        Self {
            extract_path: Arc::new(Mutex::new(None)),
        }
    }
}

impl ProcessPipeHandler for YtdlpStderrHandler {
    fn read_line(&self, line: &str) -> ControlFlow<()> {
        match ytdlp::parse_stderr_line(line) {
            None => ControlFlow::Continue(()),
            Some(ytdlp::ParsedStderrLine::MissingVideo(id)) => {
                *self.extract_path.lock().unwrap() = Some(Err(anyhow::anyhow!("Missing video id: {id}")));
                ControlFlow::Break(())
            },
            Some(ytdlp::ParsedStderrLine::UsageError(message)) => {
                *self.extract_path.lock().unwrap() = Some(Err(anyhow::anyhow!("Usage error: {message}")));
                ControlFlow::Break(())
            },
            Some(ytdlp::ParsedStderrLine::ExtractPath(path)) => {
                *self.extract_path.lock().unwrap() = Some(Ok(path));
                ControlFlow::Continue(())
            },
        }
    }

    fn finish(&self) {

    }
}

pub struct DownloadWorkers {
    database: Arc<Database>,
    threadpool: Arc<ThreadPool>,
    app_config: Arc<AppConfig>,
    cache: DashMap<VideoId, Arc<DownloadWorker>>,
}

impl DownloadWorkers {
    pub fn new(database: Arc<Database>, threadpool: Arc<ThreadPool>, app_config: Arc<AppConfig>) -> Self {
        Self {
            database,
            threadpool,
            app_config,
            cache: DashMap::new(),
        }
    }

    pub fn get_worker(&self, video_id: &VideoId) -> Option<Arc<DownloadWorker>> {
        self.cache.get(video_id).as_deref().cloned()
    }

    pub fn delete_worker(&self, video_id: &VideoId) -> Option<Arc<DownloadWorker>> {
        self.cache.remove(video_id).map(|(_key, value)| value)
    }

    pub fn start_worker(&self, video_id: &VideoId) -> anyhow::Result<Arc<DownloadWorker>> {
        // check cache hit
        if let Some(worker) = self.cache.get(video_id) {
            let state = worker.state.lock().unwrap();
            if state.worker_status.is_healthy() {
                return Ok(worker.clone());
            }
        }
        // new cache item
        let worker = Arc::new(DownloadWorker::new(video_id.clone()));
        let _old_worker = self.cache.insert(video_id.clone(), worker.clone());
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
            let threadpool = self.threadpool.clone();
            let worker = worker.clone();
            move || -> anyhow::Result<()> {
                // setup process
                let output_dirpath = app_config.downloads_folder.join(video_id.as_str());
                let mut process = ProcessWorker::new(threadpool.clone(), &output_dirpath);
                process.label = Some(format!("download_{0}", video_id.as_str()));
                let command = create_download_command(&video_id, &output_dirpath, &app_config)?;
                let stdout_handler = Box::new(YtdlpStdoutHandler::new(worker.clone()));
                let stderr_handler = Box::new(YtdlpStderrHandler::default());
                process.stdout_handler = Some(stdout_handler.clone());
                process.stderr_handler = Some(stderr_handler.clone());
                if let Err(err) = process.run(command) {
                    return Err(anyhow::anyhow!("Failed to run process: {err:?}"));
                }
                // update logging files
                let system_log_filepath = app_config.get_relative_data_path(&process.system_log_filename)?;
                let stdout_filepath = app_config.get_relative_data_path(&process.stdout_filename)?;
                let stderr_filepath = app_config.get_relative_data_path(&process.stderr_filename)?;
                database
                    .connect()?
                    .select_and_update_ytdlp_entry(&video_id, move |entry| {
                        entry.stdout_log_path = Some(stdout_filepath.to_string_lossy().to_string());
                        entry.stderr_log_path = Some(stderr_filepath.to_string_lossy().to_string());
                        entry.system_log_path = Some(system_log_filepath.to_string_lossy().to_string());
                    })?;
                // NOTE: Audio extractor for yt-dlp might not extract anything if the file extension remains the same
                let download_filepath: Option<String> = stdout_handler.download_path.lock().unwrap().clone();
                let extract_filepath: Option<String> = stderr_handler.extract_path.lock().unwrap().as_mut()
                    .and_then(|res| res.as_ref().ok().cloned());
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
                let relative_output_filepath = match app_config.get_relative_data_path(&output_filepath) {
                    Ok(path) => path,
                    Err(err) => {
                        return Err(anyhow::anyhow!("Failed to get relative output filepath: {0}", err));
                    },
                };
                log::debug!("Got ytdlp relative output filepath: {0}", relative_output_filepath.to_string_lossy());
                database
                    .connect()?
                    .select_and_update_ytdlp_entry(&video_id, move |entry| {
                        entry.audio_path = Some(relative_output_filepath.to_string_lossy().to_string());
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

        self.threadpool.execute(move || {
            if let Err(err) = outer_runner() {
                log::error!("Runner failed with: {0}", err);
            }
        });
        Ok(worker)
    }
}

fn create_download_command(video_id: &VideoId, output_dirpath: &Path, app_config: &AppConfig) -> anyhow::Result<Command> {
    let url = format!("https://www.youtube.com/watch?v={0}", video_id.as_str());
    let ytdlp_binary_path = app_config.get_absolute_binary_filepath(&PathBuf::from("yt-dlp.exe"))?;
    let ffmpeg_binary_path = app_config.get_absolute_binary_filepath(&PathBuf::from("ffmpeg.exe"))?;
    // Can't canonicalize since path doesn't exist yet
    let output_filepath = output_dirpath.join("%(id)s.%(ext)s");
    let output_filepath = std::path::absolute(&output_filepath)
        .with_context(|| format!("Failed to get absolute output filepath from: {0}", output_filepath.to_string_lossy()))?;
    let mut command = Command::new(ytdlp_binary_path);
    command.current_dir(&app_config.binaries_folder);
    command.args(ytdlp::get_ytdlp_arguments(
        url.as_str(),
        ffmpeg_binary_path.to_str().expect("Failed to turn ffmpeg binary path into UTF-8 string"),
        output_filepath.to_str().expect("Failed to turn output filepath into UTF-8 string"),
    ));
    Ok(command)
}
