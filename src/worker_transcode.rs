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

#[derive(Clone,Copy,Debug)]
pub struct TranscodeState {
    pub start_time_unix: u64,
    pub end_time_unix: u64,
    pub is_finished: bool,
    pub time_elapsed_microseconds: u64,
    pub size_bytes: usize,
    pub speed_bits: usize,
    pub speed_factor: u32,
}

impl Default for TranscodeState {
    fn default() -> Self {
        let curr_time = get_unix_time();
        Self {
            start_time_unix: curr_time,
            end_time_unix: curr_time,
            is_finished: false,
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

pub type TranscodeCache = Arc<Mutex<HashMap<TranscodeKey, TranscodeState>>>;

#[derive(Debug)]
pub enum TranscodeStartStatus {
    Started,
    Running,
    Finished,
}

#[derive(Debug)]
pub enum TranscodeStartError {
    DatabaseConnection(r2d2::Error),
    DatabaseInsert(rusqlite::Error),
    DatabaseDelete(rusqlite::Error),
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
    source: PathBuf, key: TranscodeKey, is_retranscode: bool,
    transcode_cache: TranscodeCache, app_config: AppConfig, 
    db_pool: DatabasePool, worker_thread_pool: WorkerThreadPool,
) -> Result<TranscodeStartStatus, TranscodeStartError> {
    // purge for fresh retranscode
    if is_retranscode {
        // we can only retranscode if there isn't a worker active
        let mut transcode_cache = transcode_cache.lock().unwrap();
        let is_worker_active = match transcode_cache.get(&key) {
            None => false,
            Some(transcode_state) => !transcode_state.is_finished,
        };
        if !is_worker_active {
            let _ = transcode_cache.remove(&key);
            let db_conn = db_pool.get().map_err(TranscodeStartError::DatabaseConnection)?;
            delete_worker_entry(&db_conn, &key.video_id, key.audio_ext, DB_TABLE)
                .map_err(TranscodeStartError::DatabaseDelete)?;
        }
    }
    // check if transcode in progress (cache hit)
    {
        let mut transcode_cache = transcode_cache.lock().unwrap();
        if let Some(state) = transcode_cache.get(&key) {
            if !state.is_finished {
                return Ok(TranscodeStartStatus::Running);
            } else {
                return Ok(TranscodeStartStatus::Finished);
            }
        }
        // NOTE: allow only one transcode at any given time (this is our mutex for database row)
        transcode_cache.insert(key.clone(), TranscodeState::default());
    }
    // rollback transcode cache entry if enqueue failed
    let is_queue_success = Rc::new(RefCell::new(false));
    let _revert_transcode_cache = defer({
        let is_queue_success = is_queue_success.clone();
        let key = key.clone();
        let transcode_cache = transcode_cache.clone();
        move || {
            if !*is_queue_success.borrow() {
                let _ = transcode_cache.lock().unwrap().remove(&key);
            }
        }
    });
    // check if transcode finished on disk (cache miss due to reset)
    let db_conn = db_pool.get().map_err(TranscodeStartError::DatabaseConnection)?;
    if let Ok(status) = select_worker_status(&db_conn, &key.video_id, key.audio_ext, DB_TABLE) {
        if status == WorkerStatus::Finished {
            let mut transcode_cache = transcode_cache.lock().unwrap();
            let mut transcode_state = transcode_cache.get(&key).copied().unwrap_or_default();
            transcode_state.is_finished = true;
            transcode_cache.insert(key.clone(), transcode_state);
            *is_queue_success.borrow_mut() = true;
            return Ok(TranscodeStartStatus::Finished);
        }
    }
    // start transcode worker
    let _ = insert_worker_entry(&db_conn, &key.video_id, key.audio_ext, DB_TABLE)
        .map_err(TranscodeStartError::DatabaseInsert)?;
    worker_thread_pool.lock().unwrap().execute(move || {
        let res = enqueue_transcode_worker(source, key.clone(), is_retranscode, transcode_cache, app_config, db_pool);
        match res {
            Ok(path) => log::info!("Transcoded file: {0}", path.to_string_lossy()),
            Err(err) => log::error!("Transcode failed: id={0}, err={1:?}", key.as_str(), err),
        }
    });
    *is_queue_success.borrow_mut() = true;
    Ok(TranscodeStartStatus::Started)
}

fn enqueue_transcode_worker(
    source: PathBuf, key: TranscodeKey, is_retranscode: bool, 
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
        move || {
            let is_transcoded = *is_transcoded.borrow();
            let worker_status = if is_transcoded {
                let mut transcode_cache = transcode_cache.lock().unwrap();
                let mut transcode_state = transcode_cache.get(&key).copied().unwrap_or_default();
                transcode_state.is_finished = true;
                let _ = transcode_cache.insert(key.clone(), transcode_state);
                WorkerStatus::Finished
            } else {
                let _ = transcode_cache.lock().unwrap().remove(&key);
                WorkerStatus::Failed
            };
            let db_conn = match db_pool.get() {
                Ok(db_conn) => db_conn,
                Err(err) => return log::error!("Failed to get database connection: id={0}, err={1:?}", key.as_str(), err),
            };
            if let Err(err) = update_worker_status(&db_conn, &key.video_id, key.audio_ext, worker_status, DB_TABLE) {
                return log::error!("Failed to worker status: id={0}, err={1:?}", key.as_str(), err);
            }
        }
    });
    // avoid retranscodeing file if on disk already
    if !is_retranscode && audio_path.exists() {
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
            let _ = writeln!(&mut system_log_writer.lock().unwrap(), "[error] ffmpeg failed to start: {err:?}")
                .map_err(WorkerError::SystemWriteFail)?;
            return Err(TranscodeError::LoggedFail);
        }
    };
    // update as running
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
                        let mut transcode_cache = transcode_cache.lock().unwrap(); 
                        let mut transcode_state = transcode_cache.get(&key).copied().unwrap_or_default();
                        transcode_state.update_from_ffmpeg(progress);
                        transcode_cache.insert(key.clone(), transcode_state);
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
        Ok(exit_result) => match exit_result {
            Some(exit_status) => match exit_status.code() {
                None => {},
                Some(0) => {},
                Some(code) => {
                    writeln!(&mut system_log_writer.lock().unwrap(), "[error] ffmpeg failed with bad code: {code:?}")
                        .map_err(WorkerError::SystemWriteFail)?;
                    return Err(TranscodeError::LoggedFail);
                },
            },
            None => {},
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
        let db_conn = db_pool.get().map_err(WorkerError::DatabaseConnection)?;
        let _ = update_worker_fields(
            &db_conn, &key.video_id, key.audio_ext, DB_TABLE,
            &["audio_path"], &[&audio_path.to_str().unwrap()],
        ).map_err(WorkerError::DatabaseExecute)?;
        Ok(audio_path)
    } else {
        Err(TranscodeError::MissingOutputFile(audio_path))
    }
}
