use std::cell::RefCell;
use std::io::{BufReader, BufWriter, BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use dashmap::DashMap;
use crate::app::{AppConfig, WorkerError, WorkerThreadPool};
use crate::database::{
    DatabasePool, VideoId, AudioExtension, WorkerStatus, WorkerTable, 
    update_worker_fields, insert_worker_entry, update_worker_status, select_worker_fields,
};
use crate::util::{get_unix_time, defer, ConvertCarriageReturnToNewLine};
use crate::worker_download::{DownloadCache, DOWNLOAD_AUDIO_EXT};
use crate::ffmpeg;

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct TranscodeKey {
    pub video_id: VideoId,
    pub audio_ext: AudioExtension,
}

impl TranscodeKey {
    pub fn as_str(&self) -> String {
        format!("{}.{}", self.video_id.as_str(), self.audio_ext.as_str())
    }
}

#[derive(Clone,Debug)]
pub struct TranscodeState {
    pub worker_status: Arc<(Mutex<WorkerStatus>, Condvar)>,
    pub start_time_unix: u64,
    pub end_time_unix: u64,
    pub time_elapsed_microseconds: u64,
    pub size_bytes: usize,
    pub speed_bits: usize,
    pub speed_factor: u32,
}

impl Default for TranscodeState {
    fn default() -> Self {
        let curr_time = get_unix_time();
        Self {
            worker_status: Arc::new((Mutex::new(WorkerStatus::None), Condvar::new())),
            start_time_unix: curr_time,
            end_time_unix: curr_time,
            time_elapsed_microseconds: 0,
            size_bytes: 0,
            speed_bits: 0,
            speed_factor: 0,
        }
    }
}

impl TranscodeState {
    pub fn update_from_ffmpeg(&mut self, progress: ffmpeg::TranscodeProgress) {
        self.end_time_unix = get_unix_time();
        if let Some(size_bytes) = progress.size_bytes {
            self.size_bytes = size_bytes;
        }
        if let Some(time_elapsed) = progress.time_elapsed {
            self.time_elapsed_microseconds = time_elapsed.to_microseconds();
        }
        if let Some(speed_bits) = progress.speed_bits {
            self.speed_bits = speed_bits;
        }
        if let Some(speed_factor) = progress.speed_factor {
            self.speed_factor = speed_factor;
        }
    }
}

pub type TranscodeCache = Arc<DashMap<TranscodeKey, TranscodeState>>;

#[derive(Debug)]
pub enum TranscodeStartError {
    DatabaseConnection(r2d2::Error),
    DatabaseExecute(rusqlite::Error),
}

#[derive(Debug)]
pub enum TranscodeError {
    WorkerError(WorkerError),
    UsageError(String),
    MissingOutputFile(PathBuf),
    LoggedFail,
}

impl From<WorkerError> for TranscodeError {
    fn from(err: WorkerError) -> Self {
        Self::WorkerError(err)
    }
}

const DB_TABLE: WorkerTable = WorkerTable::FFMPEG;

pub fn try_start_transcode_worker(
    key: TranscodeKey,
    download_cache: DownloadCache, transcode_cache: TranscodeCache, app_config: AppConfig, 
    db_pool: DatabasePool, worker_thread_pool: WorkerThreadPool,
) -> Result<WorkerStatus, TranscodeStartError> {
    // check if transcode in progress (cache hit)
    {
        let transcode_state = transcode_cache.entry(key.clone()).or_default();
        let mut worker_status = transcode_state.worker_status.0.lock().unwrap();
        match *worker_status {
            WorkerStatus::None | WorkerStatus::Failed => {
                *worker_status = WorkerStatus::Queued;
                transcode_state.worker_status.1.notify_all();
            },
            WorkerStatus::Queued | WorkerStatus::Running | WorkerStatus::Finished => return Ok(*worker_status),
        }
    }
    // rollback transcode cache entry if enqueue failed
    let is_queue_success = Rc::new(RefCell::new(false));
    let _revert_transcode_cache = defer({
        let is_queue_success = is_queue_success.clone();
        let key = key.clone();
        let transcode_cache = transcode_cache.clone();
        move || {
            if !*is_queue_success.borrow() {
                let transcode_state = transcode_cache.get(&key).unwrap();
                let mut worker_status = transcode_state.worker_status.0.lock().unwrap();
                *worker_status = WorkerStatus::None;
                transcode_state.worker_status.1.notify_all();
            }
        }
    });
    {
        let db_conn = db_pool.get().map_err(TranscodeStartError::DatabaseConnection)?;
        // check if transcode finished on disk (cache miss due to reset)
        let res: rusqlite::Result<(Option<WorkerStatus>, Option<String>)> = select_worker_fields(
            &db_conn, &key.video_id, key.audio_ext, DB_TABLE,
            &["status", "audio_path"],
            |row| Ok((TryFrom::<u8>::try_from(row.get(0)?).ok(), row.get(1)?)),
        );
        if let Ok((Some(status), Some(audio_path))) = res {
            let audio_path = PathBuf::from(audio_path);
            if status == WorkerStatus::Finished && audio_path.exists() {
                let transcode_state = transcode_cache.entry(key.clone()).or_default();
                *transcode_state.worker_status.0.lock().unwrap() = status;
                transcode_state.worker_status.1.notify_all();
                *is_queue_success.borrow_mut() = true;
                return Ok(status);
            }
        }
        // start transcode worker
        let _ = insert_worker_entry(&db_conn, &key.video_id, key.audio_ext, DB_TABLE)
            .map_err(TranscodeStartError::DatabaseExecute)?;
    }
    worker_thread_pool.lock().unwrap().execute(move || {
        log::info!("Queued transcode: id={0}", key.as_str());
        // revert queue if download worker fails
        let is_queued = Rc::new(RefCell::new(false)); 
        let _dequeue_transcode = defer({
            let is_queued = is_queued.clone();
            let key = key.clone();
            let transcode_cache = transcode_cache.clone();
            move || {
                let is_queued = *is_queued.borrow();
                if !is_queued {
                    let transcode_state = transcode_cache.get(&key).unwrap();
                    let mut worker_status = transcode_state.worker_status.0.lock().unwrap();
                    *worker_status = WorkerStatus::None;
                    transcode_state.worker_status.1.notify_all();
                }
            }
        });
        // wait for download worker
        {
            let download_state = download_cache.entry(key.video_id.clone()).or_default();
            let worker_status = download_state.worker_status.clone();
            // NOTE: Avoid deadlocking dashmap while transcode worker is queued
            drop(download_state);
            let mut download_lock = worker_status.0.lock().unwrap();
            loop {
                match *download_lock {
                    WorkerStatus::Failed => {
                        log::error!("Download failed so abandoning transcode: id={0}", key.as_str());
                        return;
                    },
                    WorkerStatus::Finished => {
                        log::debug!("Download finished so continuing to transcode: id={0}", key.as_str());
                        break;
                    },
                    WorkerStatus::None | WorkerStatus::Queued | WorkerStatus::Running => {},
                }
                download_lock = worker_status.1.wait(download_lock).unwrap();
            }
        }
        // get audio path
        let audio_path: Option<String> = {
            let db_conn = db_pool.get().map_err(TranscodeStartError::DatabaseConnection).unwrap();
            select_worker_fields(
                &db_conn, &key.video_id, DOWNLOAD_AUDIO_EXT, WorkerTable::YTDLP,
                &["audio_path"], |row| row.get(0),
            ).map_err(TranscodeStartError::DatabaseExecute).unwrap()
        };
        let Some(audio_path) = audio_path else {
            log::error!("Download worker succeeded by audio path is missing: id={0}", key.as_str());
            return;
        };
        let audio_path = PathBuf::from(audio_path);
        if !audio_path.exists() {
            log::error!("Download worker succeeded but file is missing: id={0}, path={1}",
                key.as_str(), audio_path.to_str().unwrap());
            return;
        }
        // launch worker
        *is_queued.borrow_mut() = true;
        drop(_dequeue_transcode);
        log::info!("Launching transcode process: {0}", key.as_str());
        let res = enqueue_transcode_worker(audio_path, key.clone(), transcode_cache, app_config, db_pool);
        match res {
            Ok(path) => log::info!("Transcode succeeded: path={0}", path.to_string_lossy()),
            Err(err) => log::error!("Transcode failed: id={0}, err={1:?}", key.as_str(), err),
        }
    });
    *is_queue_success.borrow_mut() = true;
    Ok(WorkerStatus::Queued)
}

fn enqueue_transcode_worker(
    source: PathBuf, key: TranscodeKey,
    transcode_cache: TranscodeCache, app_config: AppConfig, db_pool: DatabasePool,
) -> Result<PathBuf, TranscodeError> {
    let filename = format!("{0}.{1}", key.video_id.as_str(), key.audio_ext.as_str());
    let audio_path = app_config.transcode.join(filename.as_str());
    // update cache on exit
    let is_transcoded = Rc::new(RefCell::new(false));
    let _update_cache_and_database = defer({
        let is_transcoded = is_transcoded.clone();
        let key = key.clone();
        let transcode_cache = transcode_cache.clone();
        let db_pool = db_pool.clone();
        let audio_path = audio_path.clone();
        move || {
            let is_transcoded = *is_transcoded.borrow();
            let worker_status = if is_transcoded { WorkerStatus::Finished } else { WorkerStatus::Failed };
            let db_conn = match db_pool.get() {
                Ok(db_conn) => db_conn,
                Err(err) => return log::error!("Failed to get database connection: id={0}, err={1:?}", key.as_str(), err),
            };
            if let Err(err) = update_worker_status(&db_conn, &key.video_id, key.audio_ext, worker_status, DB_TABLE) {
                return log::error!("Failed to worker status: id={0}, err={1:?}", key.as_str(), err);
            }
            if let Err(err) = update_worker_fields(
                &db_conn, &key.video_id, key.audio_ext, DB_TABLE,
                &["audio_path"], &[&audio_path.to_str().unwrap()],
            ) {
                return log::error!("Failed to update worker audio path: id={0}, err={1:?}", key.as_str(), err);
            }
            drop(db_conn);
            // NOTE: Do this after database update so changes are immediately visible
            let transcode_state = transcode_cache.entry(key.clone()).or_default();
            *transcode_state.worker_status.0.lock().unwrap() = worker_status;
            transcode_state.worker_status.1.notify_all();
        }
    });
    // avoid retranscodeing file if on disk already
    if audio_path.exists() {
        *is_transcoded.borrow_mut() = true;
        return Ok(audio_path);
    }
    // logging files
    let stdout_log_path = app_config.transcode.join(format!("{}.stdout.log", key.as_str()));
    let stderr_log_path = app_config.transcode.join(format!("{}.stderr.log", key.as_str()));
    let system_log_path = app_config.transcode.join(format!("{}.system.log", key.as_str()));
    let system_log_file = std::fs::File::create(system_log_path.clone()).map_err(WorkerError::SystemLogCreate)?;
    let system_log_writer = Arc::new(Mutex::new(BufWriter::new(system_log_file)));
    {
        let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
        let _ = update_worker_fields(
            &db_conn, &key.video_id, key.audio_ext, DB_TABLE,
            &["system_log_path"], &[&system_log_path.to_str().unwrap()]
        ).map_err(WorkerError::DatabaseExecute)?;
    }
    // spawn process
    let process_res = Command::new(app_config.ffmpeg_binary.clone())
        .args([
            "-i", source.to_str().unwrap(),
            "-progress", "-", "-y",
            audio_path.to_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut process = match process_res {
        Ok(process) => process,
        Err(err) => {
            writeln!(&mut system_log_writer.lock().unwrap(), "[error] ffmpeg failed to start: {err:?}")
                .map_err(WorkerError::SystemWriteFail)?;
            return Err(TranscodeError::LoggedFail);
        }
    };
    // update as running
    {
        let transcode_state = transcode_cache.get(&key).unwrap();
        *transcode_state.worker_status.0.lock().unwrap() = WorkerStatus::Running;
        transcode_state.worker_status.1.notify_all();
    }
    {
        let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
        let _ = update_worker_status(&db_conn, &key.video_id, key.audio_ext, WorkerStatus::Running, DB_TABLE)
            .map_err(WorkerError::DatabaseExecute)?;
    }
    // scrape stdout and stderr
    let stdout_thread = thread::spawn({
        let db_pool = db_pool.clone();
        let key = key.clone();
        let stdout_handle = process.stdout.take().ok_or(WorkerError::StdoutMissing)?;
        let mut stdout_reader = BufReader::new(ConvertCarriageReturnToNewLine::new(stdout_handle));
        let stdout_log_file = std::fs::File::create(stdout_log_path.clone()).map_err(WorkerError::StdoutLogCreate)?;
        let mut stdout_log_writer = BufWriter::new(stdout_log_file);
        {
            let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
            let _ = update_worker_fields(
                &db_conn, &key.video_id, key.audio_ext, DB_TABLE,
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
                line.clear();
            }
            Ok(())
        }
    });
    let stderr_thread = thread::spawn({
        let db_pool = db_pool.clone();
        let key = key.clone();
        let stderr_handle = process.stderr.take().ok_or(WorkerError::StderrMissing)?;
        let mut stderr_reader = BufReader::new(ConvertCarriageReturnToNewLine::new(stderr_handle));
        let stderr_log_file = std::fs::File::create(stderr_log_path.clone()).map_err(WorkerError::StderrLogCreate)?;
        let mut stderr_log_writer = BufWriter::new(stderr_log_file);
        {
            let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
            let _ = update_worker_fields(
                &db_conn, &key.video_id, key.audio_ext, DB_TABLE,
                &["stderr_log_path"], &[&stderr_log_path.to_str().unwrap()],
            ).map_err(WorkerError::DatabaseExecute)?;
        }
        move || -> Result<(), WorkerError> {
            let mut line = String::new();
            loop {
                match stderr_reader.read_line(&mut line) {
                    Err(_) => break,
                    Ok(0) => break,
                    Ok(_) => (),
                }
                let _ = stderr_log_writer.write(line.as_bytes()).map_err(WorkerError::StderrWriteFail)?;
                match ffmpeg::parse_stderr_line(line.as_str()) {
                    None => (),
                    Some(ffmpeg::ParsedStderrLine::TranscodeProgress(progress)) => {
                        log::debug!("[transcode] id={0} progress={progress:?}", key.as_str());
                        let mut transcode_state = transcode_cache.entry(key.clone()).or_default();
                        transcode_state.update_from_ffmpeg(progress);
                    },
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
                writeln!(&mut system_log_writer.lock().unwrap(), "[error] ffmpeg failed with bad code: {code:?}")
                    .map_err(WorkerError::SystemWriteFail)?;
                return Err(TranscodeError::LoggedFail);
            },
        },
        Err(err) => {
            writeln!(&mut system_log_writer.lock().unwrap(), "[warn] ffmpeg process failed to join: {err:?}")
                .map_err(WorkerError::SystemWriteFail)?;
            if let Err(err) = process.kill() {
                writeln!(&mut system_log_writer.lock().unwrap(), "[warn] ffmpeg process failed to be killed: {err:?}")
                    .map_err(WorkerError::SystemWriteFail)?;
            }
        },
    }
    let audio_file_transcoded = audio_path.exists();
    *is_transcoded.borrow_mut() = audio_file_transcoded;
    if audio_file_transcoded {
        Ok(audio_path)
    } else {
        Err(TranscodeError::MissingOutputFile(audio_path))
    }
}
