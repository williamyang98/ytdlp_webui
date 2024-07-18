use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufReader, BufWriter, BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::app::{AppConfig, WorkerError, WorkerThreadPool};
use crate::database::{
    DatabasePool, VideoId, AudioExtension, WorkerStatus, WorkerTable,
    delete_worker_entry, update_worker_fields, insert_worker_entry,
    update_worker_status, select_worker_status,
};
use crate::util::{get_unix_time, defer, ConvertCarriageReturnToNewLine};
use crate::ytdlp;

#[derive(Clone,Copy,Debug)]
pub struct DownloadState {
    pub is_finished: bool,
    pub start_time_unix: u64,
    pub end_time_unix: u64,
    pub downloaded_bytes: usize,
    pub size_bytes: usize,
    pub speed_bytes: usize,
}

impl Default for DownloadState {
    fn default() -> Self {
        let curr_time = get_unix_time();
        Self {
            start_time_unix: curr_time,
            end_time_unix: curr_time,
            is_finished: false,
            downloaded_bytes: 0,
            size_bytes: 0,
            speed_bytes: 0,
        }
    }
}

impl DownloadState {
    pub fn update_from_ytdlp(&mut self, progress: ytdlp::DownloadProgress) {
        self.end_time_unix = get_unix_time();
        if let Some(size_bytes) = progress.size_bytes {
            self.size_bytes = size_bytes;
            if let Some(percentage) = progress.percentage {
                let total_bytes = (size_bytes as f32 * percentage * 0.01) as usize;
                self.downloaded_bytes = total_bytes;
            }
        }
        if let Some(speed_bytes) = progress.speed_bytes {
            self.speed_bytes = speed_bytes;
        }
    }
}

pub type DownloadCache = Arc<Mutex<HashMap<VideoId, DownloadState>>>;
pub const DOWNLOAD_AUDIO_EXT: AudioExtension = AudioExtension::M4A;

#[derive(Debug)]
pub enum DownloadStartStatus {
    Started,
    Running,
    Finished,
}

#[derive(Debug)]
pub enum DownloadStartError {
    DatabaseConnection(r2d2::Error),
    DatabaseInsert(rusqlite::Error),
    DatabaseDelete(rusqlite::Error),
}

#[derive(Debug)]
pub enum DownloadError {
    WorkerError(WorkerError),
    UsageError(String),
    InvalidVideoId,
    MissingOutputFile(PathBuf),
    LoggedFail,
}

impl From<WorkerError> for DownloadError {
    fn from(err: WorkerError) -> Self {
        Self::WorkerError(err)
    }
}

const DB_TABLE: WorkerTable = WorkerTable::YTDLP;

pub fn try_start_download_worker(
    video_id: VideoId, is_redownload: bool,
    download_cache: DownloadCache, app_config: AppConfig, 
    db_pool: DatabasePool, worker_thread_pool: WorkerThreadPool,
) -> Result<DownloadStartStatus, DownloadStartError> {
    // purge for fresh redownload
    if is_redownload {
        // we can only redownload if there isn't a worker active
        let mut download_cache = download_cache.lock().unwrap();
        let is_worker_active = match download_cache.get(&video_id) {
            None => false,
            Some(download_state) => !download_state.is_finished,
        };
        if !is_worker_active {
            let _ = download_cache.remove(&video_id);
            let db_conn = db_pool.get().map_err(DownloadStartError::DatabaseConnection)?;
            delete_worker_entry(&db_conn, &video_id, DOWNLOAD_AUDIO_EXT, DB_TABLE)
                .map_err(DownloadStartError::DatabaseDelete)?;
        }
    }
    // check if download in progress (cache hit)
    {
        let mut download_cache = download_cache.lock().unwrap();
        if let Some(state) = download_cache.get(&video_id) {
            if !state.is_finished {
                return Ok(DownloadStartStatus::Running);
            } else {
                return Ok(DownloadStartStatus::Finished);
            }
        }
        // NOTE: allow only one download at any given time (this is our mutex for database row)
        download_cache.insert(video_id.clone(), DownloadState::default());
    }
    // rollback download cache entry if enqueue failed
    let is_queue_success = Rc::new(RefCell::new(false));
    let _revert_download_cache = defer({
        let is_queue_success = is_queue_success.clone();
        let video_id = video_id.clone();
        let download_cache = download_cache.clone();
        move || {
            if !*is_queue_success.borrow() {
                let _ = download_cache.lock().unwrap().remove(&video_id);
            }
        }
    });
    // check if download finished on disk (cache miss due to reset)
    let db_conn = db_pool.get().map_err(DownloadStartError::DatabaseConnection)?;
    if let Ok(status) = select_worker_status(&db_conn, &video_id, DOWNLOAD_AUDIO_EXT, DB_TABLE) {
        if status == WorkerStatus::Finished {
            let mut download_cache = download_cache.lock().unwrap();
            let mut download_state = download_cache.get(&video_id).copied().unwrap_or_default();
            download_state.is_finished = true;
            download_cache.insert(video_id.clone(), download_state);
            *is_queue_success.borrow_mut() = true;
            return Ok(DownloadStartStatus::Finished);
        }
    }
    // start download worker
    let _ = insert_worker_entry(&db_conn, &video_id, DOWNLOAD_AUDIO_EXT, DB_TABLE)
        .map_err(DownloadStartError::DatabaseInsert)?;
    worker_thread_pool.lock().unwrap().execute(move || {
        let res = enqueue_download_worker(video_id.clone(), is_redownload, download_cache, app_config, db_pool);
        match res {
            Ok(path) => log::info!("Downloaded file: {0}", path.to_string_lossy()),
            Err(err) => log::error!("Download failed: id={0}, err={1:?}", video_id.as_str(), err),
        }
    });
    *is_queue_success.borrow_mut() = true;
    Ok(DownloadStartStatus::Started)
}

fn enqueue_download_worker(
    video_id: VideoId, is_redownload: bool, 
    download_cache: DownloadCache, app_config: AppConfig, db_pool: DatabasePool,
) -> Result<PathBuf, DownloadError> {
    let audio_ext = DOWNLOAD_AUDIO_EXT;
    let filename = format!("{0}.{1}", video_id.as_str(), audio_ext.as_str());
    let audio_path = app_config.download.join(filename.as_str());
    // update cache on exit
    let is_downloaded = Rc::new(RefCell::new(false));
    let _update_cache_and_database = defer({
        let is_downloaded = is_downloaded.clone();
        let video_id = video_id.clone();
        let download_cache = download_cache.clone();
        let db_pool = db_pool.clone();
        move || {
            let is_downloaded = *is_downloaded.borrow();
            let worker_status = if is_downloaded {
                let mut download_cache = download_cache.lock().unwrap();
                let mut download_state = download_cache.get(&video_id).copied().unwrap_or_default();
                download_state.is_finished = true;
                let _ = download_cache.insert(video_id.clone(), download_state);
                WorkerStatus::Finished
            } else {
                let _ = download_cache.lock().unwrap().remove(&video_id);
                WorkerStatus::Failed
            };
            let db_conn = match db_pool.get() {
                Ok(db_conn) => db_conn,
                Err(err) => return log::error!("Failed to get database connection: id={0}, err={1:?}", video_id.as_str(), err),
            };
            if let Err(err) = update_worker_status(&db_conn, &video_id, audio_ext, worker_status, DB_TABLE) {
                return log::error!("Failed to worker status: id={0}, err={1:?}", video_id.as_str(), err);
            }
        }
    });
    // avoid redownloading file if on disk already
    if !is_redownload && audio_path.exists() {
        *is_downloaded.borrow_mut() = true;
        return Ok(audio_path);
    }
    // logging files
    let stdout_log_path = app_config.download.join(format!("{}.stdout.log", video_id.as_str()));
    let stderr_log_path = app_config.download.join(format!("{}.stderr.log", video_id.as_str()));
    let system_log_path = app_config.download.join(format!("{}.system.log", video_id.as_str()));
    let system_log_file = std::fs::File::create(system_log_path.clone()).map_err(WorkerError::SystemLogCreate)?;
    let system_log_writer = Arc::new(Mutex::new(BufWriter::new(system_log_file)));
    {
        let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
        let _ = update_worker_fields(
            &db_conn, &video_id, audio_ext, DB_TABLE,
            &["system_log_path"], &[&system_log_path.to_str().unwrap()]
        ).map_err(WorkerError::DatabaseExecute)?;
    }
    // spawn process
    let url = format!("https://www.youtube.com/watch?v={0}", video_id.as_str());
    let process_res = Command::new(app_config.ytdlp_binary.clone())
        .args([
            url.as_str(), 
            "-x",
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
            let _ = writeln!(&mut system_log_writer.lock().unwrap(), "[error] ytdlp failed to start: {err:?}")
                .map_err(WorkerError::SystemWriteFail)?;
            return Err(DownloadError::LoggedFail);
        }
    };
    // update as running
    {
        let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
        let _ = update_worker_status(&db_conn, &video_id, audio_ext, WorkerStatus::Running, DB_TABLE)
            .map_err(WorkerError::DatabaseExecute)?;
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
        // let system_log_writer = system_log_writer.clone();
        {
            let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
            let _ = update_worker_fields(
                &db_conn, &video_id, audio_ext, DB_TABLE,
                &["stdout_log_path"], &[&stdout_log_path.to_str().unwrap()],
            ).map_err(WorkerError::DatabaseExecute)?;
        }
        move || -> Result<(), WorkerError> {
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
                        let mut download_cache = download_cache.lock().unwrap();
                        let mut download_state = download_cache.get(&video_id).copied().unwrap_or_default();
                        download_state.update_from_ytdlp(progress);
                        download_cache.insert(video_id.clone(), download_state);
                    },
                    Some(ytdlp::ParsedStdoutLine::InfoJsonPath(path)) => {
                        let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
                        let infojson_path = app_config.root.join(path);
                        let _ = update_worker_fields(
                            &db_conn, &video_id, audio_ext, DB_TABLE,
                            &["infojson_path"], &[&infojson_path.to_str().unwrap()],
                        ).map_err(WorkerError::DatabaseExecute)?;
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
            let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
            let _ = update_worker_fields(
                &db_conn, &video_id, audio_ext, DB_TABLE,
                &["stderr_log_path"], &[&stderr_log_path.to_str().unwrap()],
            ).map_err(WorkerError::DatabaseExecute)?;
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
        Ok(exit_result) => match exit_result {
            Some(exit_status) => match exit_status.code() {
                None => {},
                Some(0) => {},
                Some(code) => {
                    writeln!(&mut system_log_writer.lock().unwrap(), "[error] ytdlp failed with bad code: {code:?}")
                        .map_err(WorkerError::SystemWriteFail)?;
                    return Err(DownloadError::LoggedFail);
                },
            },
            None => {},
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
    let audio_file_downloaded = audio_path.exists();
    *is_downloaded.borrow_mut() = audio_file_downloaded;
    if audio_file_downloaded {
        let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
        let _ = update_worker_fields(
            &db_conn, &video_id, audio_ext, DB_TABLE,
            &["audio_path"], &[&audio_path.to_str().unwrap()],
        ).map_err(WorkerError::DatabaseExecute)?;
        Ok(audio_path)
    } else {
        Err(DownloadError::MissingOutputFile(audio_path))
    }
}
