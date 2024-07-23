use std::cell::RefCell;
use std::io::{BufReader, BufWriter, BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use dashmap::DashMap;
use num_traits::cast::{FromPrimitive,ToPrimitive};
use serde::Serialize;
use thiserror::Error;
use crate::app::{AppConfig, WorkerError, WorkerThreadPool, WorkerCacheEntry};
use crate::database::{
    DatabasePool, VideoId, AudioExtension, WorkerStatus, WorkerTable, 
    update_worker_fields, insert_worker_entry, update_worker_status, select_worker_fields,
};
use crate::util::{get_unix_time, defer, ConvertCarriageReturnToNewLine};
use crate::ytdlp;

#[derive(Clone,Debug,Serialize)]
pub struct DownloadState {
    pub worker_status: WorkerStatus,
    pub fail_reason: Option<String>,
    pub start_time_unix: u64,
    pub end_time_unix: u64,
    pub percentage: Option<f32>,
    pub downloaded_bytes: Option<usize>,
    pub size_bytes: Option<usize>,
    pub speed_bytes: Option<usize>,
    pub eta: Option<ytdlp::Eta>,
}

impl Default for DownloadState {
    fn default() -> Self {
        let curr_time = get_unix_time();
        Self {
            worker_status: WorkerStatus::None,
            fail_reason: None,
            start_time_unix: curr_time,
            end_time_unix: curr_time,
            percentage: None,
            downloaded_bytes: None,
            size_bytes: None,
            speed_bytes: None,
            eta: None,
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
        update_field(&mut self.percentage, progress.percentage);
        update_field(&mut self.size_bytes, progress.size_bytes);
        if let Some(size_bytes) = progress.size_bytes {
            if let Some(percentage) = progress.percentage {
                let total_bytes = (size_bytes as f32 * percentage * 0.01) as usize;
                self.downloaded_bytes = Some(total_bytes);
            }
        }
        update_field(&mut self.speed_bytes, progress.speed_bytes);
        update_field(&mut self.eta, progress.eta);
    }
}

pub type DownloadCache = Arc<DashMap<VideoId, WorkerCacheEntry<DownloadState>>>;
pub const DOWNLOAD_AUDIO_EXT: AudioExtension = AudioExtension::M4A;

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
    #[error("Missing output download file: {0}")]
    MissingOutputFile(PathBuf),
    #[error("Error stored in system log")]
    LoggedFail,
    #[error("Database connection failed: {0:?}")]
    DatabaseConnection(#[from] r2d2::Error),
    #[error("Database execute failed: {0:?}")]
    DatabaseExecute(#[from] rusqlite::Error),
}

const DB_TABLE: WorkerTable = WorkerTable::YTDLP;

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
        let res: Option<(Option<WorkerStatus>, Option<String>)> = select_worker_fields(
            &db_conn, &video_id, DOWNLOAD_AUDIO_EXT, DB_TABLE,
            &["status", "audio_path"],
            |row| Ok((WorkerStatus::from_u8(row.get(0)?), row.get(1)?)),
        )?;
        if let Some((Some(status), Some(audio_path))) = res {
            let audio_path = PathBuf::from(audio_path);
            if status == WorkerStatus::Finished && audio_path.exists() {
                let download_state = download_cache.entry(video_id.clone()).or_default();
                download_state.0.lock().unwrap().worker_status = status;
                download_state.1.notify_all();
                *is_queue_success.borrow_mut() = true;
                return Ok(status);
            }
        }
        // start download worker
        let _ = insert_worker_entry(&db_conn, &video_id, DOWNLOAD_AUDIO_EXT, DB_TABLE)?;
    }
    worker_thread_pool.lock().unwrap().execute(move || {
        log::info!("Launching download process: {0}.{1}", video_id.as_str(), DOWNLOAD_AUDIO_EXT.as_str());
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
            update_worker_fields(
                &db_conn, &video_id, DOWNLOAD_AUDIO_EXT, DB_TABLE,
                &["system_log_path"], &[&system_log_path.to_str().unwrap()],
            ).unwrap();
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
        let db_conn = db_pool.get().unwrap();
        update_worker_fields(
            &db_conn, &video_id, DOWNLOAD_AUDIO_EXT, DB_TABLE,
            &["audio_path", "status"], 
            &[&audio_path.map(|p| p.to_str().unwrap().to_string()), &worker_status.to_u8().unwrap()],
        ).unwrap();
        drop(db_conn);
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
    let audio_ext = DOWNLOAD_AUDIO_EXT;
    let filename = format!("{0}.{1}", video_id.as_str(), audio_ext.as_str());
    let audio_path = app_config.download.join(filename.as_str());
    // TODO: avoid redownloading file if on disk already - make this an option
    // if audio_path.exists() {
    //     *is_downloaded.borrow_mut() = true;
    //     return Ok(audio_path);
    // }
    // logging files
    let stdout_log_path = app_config.download.join(format!("{}.stdout.log", video_id.as_str()));
    let stderr_log_path = app_config.download.join(format!("{}.stderr.log", video_id.as_str()));
    // spawn process
    let url = format!("https://www.youtube.com/watch?v={0}", video_id.as_str());
    let process_res = Command::new(app_config.ytdlp_binary.clone())
        .args([
            url.as_str(), 
            "--no-continue", // override existing files
            "--extract-audio",
            "--audio-format", audio_ext.into(),
            "--write-info-json",
            "--output", audio_path.to_str().unwrap(),
        ])
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
        let _ = update_worker_status(&db_conn, &video_id, audio_ext, WorkerStatus::Running, DB_TABLE)?;
    }
    // scrape stdout and stderr
    let stdout_thread = thread::spawn({
        let db_pool = db_pool.clone();
        let video_id = video_id.clone();
        let app_config = app_config.clone();
        let stdout_handle = process.stdout.take().ok_or(WorkerError::StdoutMissing)?;
        let mut stdout_reader = BufReader::new(ConvertCarriageReturnToNewLine::new(stdout_handle));
        let stdout_log_file = std::fs::File::create(stdout_log_path.clone()).map_err(WorkerError::StdoutLogCreate)?;
        let mut stdout_log_writer = BufWriter::new(stdout_log_file);
        {
            let db_conn = db_pool.get()?;
            let _ = update_worker_fields(
                &db_conn, &video_id, audio_ext, DB_TABLE,
                &["stdout_log_path"], &[&stdout_log_path.to_str().unwrap()],
            )?;
        }
        move || -> Result<(), DownloadError> {
            let mut line = String::new();
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
                        log::debug!("[download] id={0}.{1} progress={progress:?}", video_id.as_str(), audio_ext.as_str());
                        let download_state = download_cache.entry(video_id.clone()).or_default();
                        download_state.0.lock().unwrap().update_from_ytdlp(progress);
                    },
                    Some(ytdlp::ParsedStdoutLine::InfoJsonPath(path)) => {
                        let db_conn = db_pool.get()?;
                        let infojson_path = app_config.root.join(path);
                        let _ = update_worker_fields(
                            &db_conn, &video_id, audio_ext, DB_TABLE,
                            &["infojson_path"], &[&infojson_path.to_str().unwrap()],
                        )?;
                    },
                }
                line.clear();
            }
            Ok(())
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
            let _ = update_worker_fields(
                &db_conn, &video_id, audio_ext, DB_TABLE,
                &["stderr_log_path"], &[&stderr_log_path.to_str().unwrap()],
            )?;
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
    stdout_thread.join().map_err(WorkerError::StdoutThreadJoin)??;
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
    if audio_path.exists() {
        Ok(audio_path)
    } else {
        Err(DownloadError::MissingOutputFile(audio_path))
    }
}
