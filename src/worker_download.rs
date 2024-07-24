use std::cell::RefCell;
use std::io::{BufReader, BufWriter, BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use dashmap::DashMap;
use serde::Serialize;
use thiserror::Error;
use crate::app::{AppConfig, WorkerError, WorkerThreadPool, WorkerCacheEntry};
use crate::database::{
    DatabasePool, VideoId, WorkerStatus,
    insert_ytdlp_entry, select_ytdlp_entry, select_and_update_ytdlp_entry,
};
use crate::util::{get_unix_time, defer, ConvertCarriageReturnToNewLine};
use crate::ytdlp;

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
            worker_status: WorkerStatus::None,
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
    pub fn update_from_ytdlp(&mut self, progress: ytdlp::DownloadProgress) {
        self.end_time_unix = get_unix_time();
        update_field(&mut self.eta_seconds, progress.eta_seconds);
        update_field(&mut self.elapsed_seconds, progress.elapsed_seconds);
        update_field(&mut self.downloaded_bytes, progress.downloaded_bytes);
        update_field(&mut self.total_bytes, progress.total_bytes);
        update_field(&mut self.speed_bytes, progress.speed_bytes);
    }
}

pub type DownloadCache = Arc<DashMap<VideoId, WorkerCacheEntry<DownloadState>>>;

#[derive(Debug,Error)]
pub enum DownloadStartError {
    #[error("Database connection failed: {0:?}")]
    DatabaseConnection(#[from] r2d2::Error),
    #[error("Database execute failed: {0:?}")]
    DatabaseExecute(#[from] rusqlite::Error),
}

#[derive(Debug,Error)]
pub enum DownloadError {
    #[error("Worker error: {0}")]
    WorkerError(#[from] WorkerError),
    #[error("Usage error: {0}")]
    UsageError(String),
    #[error("Invalid video id")]
    InvalidVideoId,
    #[error("Missing output path")]
    MissingOutputPath,
    #[error("Missing output download file: {0}")]
    MissingOutputFile(PathBuf),
    #[error("Error stored in system log")]
    LoggedFail,
    #[error("Database connection failed: {0:?}")]
    DatabaseConnection(#[from] r2d2::Error),
    #[error("Database execute failed: {0:?}")]
    DatabaseExecute(#[from] rusqlite::Error),
}

pub fn try_start_download_worker(
    video_id: VideoId, download_cache: DownloadCache, app_config: AppConfig,
    db_pool: DatabasePool, worker_thread_pool: WorkerThreadPool,
) -> Result<WorkerStatus, DownloadStartError> {
    // check if download in progress (cache hit)
    {
        let download_state = download_cache.entry(video_id.clone()).or_default();
        let mut state = download_state.0.lock().unwrap();
        match state.worker_status {
            WorkerStatus::None | WorkerStatus::Failed => {
                state.worker_status = WorkerStatus::Queued;
                download_state.1.notify_all();
            },
            WorkerStatus::Queued | WorkerStatus::Running | WorkerStatus::Finished => return Ok(state.worker_status),
        }
    }
    // rollback download cache entry if enqueue failed
    let is_queue_success = Rc::new(RefCell::new(false));
    let _revert_download_cache = defer({
        let is_queue_success = is_queue_success.clone();
        let video_id = video_id.clone();
        let download_cache = download_cache.clone();
        move || {
            if !*is_queue_success.borrow() {
                let download_state = download_cache.get(&video_id).unwrap();
                download_state.0.lock().unwrap().worker_status = WorkerStatus::None;
                download_state.1.notify_all();
            }
        }
    });
    {
        let db_conn = db_pool.get()?;
        // check if download finished on disk (cache miss due to reset)
        let entry = select_ytdlp_entry(&db_conn, &video_id)?;
        if let Some(entry) = entry {
            if let Some(audio_path) = entry.audio_path {
                let status = entry.status;
                let audio_path = PathBuf::from(audio_path);
                if status == WorkerStatus::Finished && audio_path.exists() {
                    let download_state = download_cache.entry(video_id.clone()).or_default();
                    let mut state = download_state.0.lock().unwrap();
                    state.worker_status = status;
                    state.file_cached = true;
                    download_state.1.notify_all();
                    *is_queue_success.borrow_mut() = true;
                    return Ok(status);
                }
            }
        }
        // start download worker
        let _ = insert_ytdlp_entry(&db_conn, &video_id)?;
    }
    worker_thread_pool.lock().unwrap().execute(move || {
        log::info!("Launching download process: {0}", video_id.as_str());
        // setup logging
        let system_log_path = app_config.download.join(format!("{}.system.log", video_id.as_str()));
        let system_log_file = match std::fs::File::create(system_log_path.clone()) {
            Ok(system_log_file) => system_log_file,
            Err(err) => {
                log::error!("Failed to create system log file: path={0}, err={1:?}", system_log_path.to_str().unwrap(), err);
                return;
            },
        };
        if let Ok(db_conn) = db_pool.get() {
            select_and_update_ytdlp_entry(&db_conn, &video_id, |entry| {
                entry.system_log_path = Some(system_log_path.to_str().unwrap().to_owned());
            }).unwrap();
        }
        let system_log_writer = Arc::new(Mutex::new(BufWriter::new(system_log_file)));
        // launch process
        let res = enqueue_download_worker(
            video_id.clone(), download_cache.clone(), app_config.clone(), db_pool.clone(), system_log_writer.clone(),
        );
        if let Err(ref err) = res {
            let _ = writeln!(&mut system_log_writer.lock().unwrap(), "[error] Worker failed with: {err:?}");
        }
        // update database
        let (audio_path, worker_status, worker_error) = match res {
            Ok(path) => (Some(path), WorkerStatus::Finished, None),
            Err(err) => (None, WorkerStatus::Failed, Some(err)),
        };
        {
            let db_conn = db_pool.get().unwrap();
            let _ = select_and_update_ytdlp_entry(&db_conn, &video_id, |entry| {
                entry.audio_path = audio_path.map(|p| p.to_str().unwrap().to_string());
                entry.status = worker_status;
            }).unwrap();
        }
        // NOTE: update cache so changes to database are visible to signal listeners (transcode threads)
        let download_state = download_cache.entry(video_id.clone()).or_default();
        let mut state = download_state.0.lock().unwrap();
        state.worker_status = worker_status;
        state.fail_reason = worker_error.map(|e| e.to_string());
        download_state.1.notify_all();
    });
    *is_queue_success.borrow_mut() = true;
    Ok(WorkerStatus::Queued)
}

fn enqueue_download_worker(
    video_id: VideoId, download_cache: DownloadCache, app_config: AppConfig, db_pool: DatabasePool,
    system_log_writer: Arc<Mutex<impl Write>>,
) -> Result<PathBuf, DownloadError> {
    // logging files
    let stdout_log_path = app_config.download.join(format!("{}.stdout.log", video_id.as_str()));
    let stderr_log_path = app_config.download.join(format!("{}.stderr.log", video_id.as_str()));
    // spawn process
    let url = format!("https://www.youtube.com/watch?v={0}", video_id.as_str());
    let process_res = Command::new(app_config.ytdlp_binary.clone())
        .args(ytdlp::get_ytdlp_arguments(
            url.as_str(), 
            app_config.ffmpeg_binary.to_str().unwrap(),
            app_config.download.join("%(id)s.%(ext)s").to_str().unwrap(),
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut process = match process_res {
        Ok(process) => process,
        Err(err) => {
            writeln!(&mut system_log_writer.lock().unwrap(), "[error] ytdlp failed to start: {err:?}")
                .map_err(WorkerError::SystemWriteFail)?;
            return Err(DownloadError::LoggedFail);
        }
    };
    // update as running
    {
        let download_state = download_cache.get(&video_id).unwrap();
        download_state.0.lock().unwrap().worker_status = WorkerStatus::Running;
        download_state.1.notify_all();
    }
    {
        let db_conn = db_pool.get()?;
        let _ = select_and_update_ytdlp_entry(&db_conn, &video_id, |entry| entry.status = WorkerStatus::Running)?;
    }
    // scrape stdout and stderr
    let stdout_thread = thread::spawn({
        let db_pool = db_pool.clone();
        let video_id = video_id.clone();
        let stdout_handle = process.stdout.take().ok_or(WorkerError::StdoutMissing)?;
        let mut stdout_reader = BufReader::new(ConvertCarriageReturnToNewLine::new(stdout_handle));
        let stdout_log_file = std::fs::File::create(stdout_log_path.clone()).map_err(WorkerError::StdoutLogCreate)?;
        let mut stdout_log_writer = BufWriter::new(stdout_log_file);
        {
            let db_conn = db_pool.get()?;
            let _ = select_and_update_ytdlp_entry(&db_conn, &video_id, |entry| {
                entry.stdout_log_path = Some(stdout_log_path.to_str().unwrap().to_owned());
            })?;
        }
        move || -> Result<Option<String>, DownloadError> {
            let mut line = String::new();
            let mut output_path = None;
            loop {
                match stdout_reader.read_line(&mut line) {
                    Err(_) => break,
                    Ok(0) => break,
                    Ok(_) => (),
                }
                let _ = stdout_log_writer.write(line.as_bytes()).map_err(WorkerError::StdoutWriteFail)?;
                match ytdlp::parse_stdout_line(line.as_str()) {
                    None => (),
                    Some(ytdlp::ParsedStdoutLine::DownloadProgress(progress)) => {
                        log::debug!("[download] id={0} progress={progress:?}", video_id.as_str());
                        let download_state = download_cache.entry(video_id.clone()).or_default();
                        download_state.0.lock().unwrap().update_from_ytdlp(progress);
                    },
                    Some(ytdlp::ParsedStdoutLine::OutputPath(path)) => {
                        output_path = Some(path);
                    },
                }
                line.clear();
            }
            Ok(output_path)
        }
    });
    let stderr_thread = thread::spawn({
        let db_pool = db_pool.clone();
        let video_id = video_id.clone();
        let stderr_handle = process.stderr.take().ok_or(WorkerError::StderrMissing)?;
        let mut stderr_reader = BufReader::new(ConvertCarriageReturnToNewLine::new(stderr_handle));
        let stderr_log_file = std::fs::File::create(stderr_log_path.clone()).map_err(WorkerError::StderrLogCreate)?;
        let mut stderr_log_writer = BufWriter::new(stderr_log_file);
        {
            let db_conn = db_pool.get()?;
            let _ = select_and_update_ytdlp_entry(&db_conn, &video_id, |entry| {
                entry.stderr_log_path = Some(stderr_log_path.to_str().unwrap().to_owned());
            })?;
        }
        move || {
            let mut line = String::new();
            loop {
                match stderr_reader.read_line(&mut line) {
                    Err(_) => break,
                    Ok(0) => break,
                    Ok(_) => (),
                }
                let _ = stderr_log_writer.write(line.as_bytes()).map_err(WorkerError::StderrWriteFail)?;
                match ytdlp::parse_stderr_line(line.as_str()) {
                    None => (),
                    Some(ytdlp::ParsedStderrLine::MissingVideo(_)) => return Err(DownloadError::InvalidVideoId),
                    Some(ytdlp::ParsedStderrLine::UsageError(message)) => return Err(DownloadError::UsageError(message)),
                }
                line.clear();
            }
            Ok(())
        }
    });
    // shutdown threads
    let audio_path = stdout_thread.join().map_err(WorkerError::StdoutThreadJoin)??;
    stderr_thread.join().map_err(WorkerError::StderrThreadJoin)??;
    // shutdown process
    match process.try_wait() {
        Ok(None) => {},
        Ok(Some(exit_status)) => match exit_status.code() {
            None => {},
            Some(0) => {},
            Some(code) => {
                writeln!(&mut system_log_writer.lock().unwrap(), "[error] ytdlp failed with bad code: {code:?}")
                    .map_err(WorkerError::SystemWriteFail)?;
                return Err(DownloadError::LoggedFail);
            },
        },
        Err(err) => {
            writeln!(&mut system_log_writer.lock().unwrap(), "[warn] ytdlp process failed to join: {err:?}")
                .map_err(WorkerError::SystemWriteFail)?;
            if let Err(err) = process.kill() {
                writeln!(&mut system_log_writer.lock().unwrap(), "[warn] ytdlp process failed to be killed: {err:?}")
                    .map_err(WorkerError::SystemWriteFail)?;
            }
        },
    }
    let Some(audio_path) = audio_path else {
        return Err(DownloadError::MissingOutputPath)
    };
    let audio_path = app_config.root.join(audio_path);
    if audio_path.exists() {
        Ok(audio_path)
    } else {
        Err(DownloadError::MissingOutputFile(audio_path))
    }
}
