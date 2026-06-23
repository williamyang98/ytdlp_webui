use crate::app::AppThreadPool;
use crate::app_config::AppConfig;
use crate::database::{TranscodeKey, Database, WorkerStatus};
use crate::ffmpeg;
use crate::util::get_unix_time;
use crate::worker_download::DownloadWorkers;
use crate::process::Process;
use youtube_api::VideoItem;
use derive_more::Debug;
use serde::Serialize;
use anyhow::Context;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use uuid::Uuid;

#[derive(Debug,Clone,Serialize)]
pub struct TranscodeState {
    pub id: Uuid,
    pub worker_status: WorkerStatus,
    pub file_cached: bool,
    pub fail_reason: Option<String>,
    pub start_time_unix: u64,
    pub end_time_unix: u64,
    pub source_duration_milliseconds: Option<u64>,
    pub source_start_time_milliseconds: Option<u64>,
    pub source_speed_bits: Option<usize>,
    pub transcode_duration_milliseconds: Option<u64>,
    pub transcode_size_bytes: Option<usize>,
    pub transcode_speed_bits: Option<usize>,
    pub transcode_speed_factor: Option<f32>,
}

impl Default for TranscodeState {
    fn default() -> Self {
        let curr_time = get_unix_time();
        Self {
            id: Uuid::new_v4(),
            worker_status: WorkerStatus::default(),
            file_cached: false,
            fail_reason: None,
            start_time_unix: curr_time,
            end_time_unix: curr_time,
            source_duration_milliseconds: None,
            source_start_time_milliseconds: None,
            source_speed_bits: None,
            transcode_duration_milliseconds: None,
            transcode_size_bytes: None,
            transcode_speed_bits: None,
            transcode_speed_factor: None,
        }
    }
}

fn update_field<T>(dst: &mut Option<T>, src: Option<T>) {
    if src.is_some() {
        *dst = src;
    }
}

impl TranscodeState {
    pub fn update_from_progress(&mut self, progress: &ffmpeg::TranscodeProgress) {
        self.end_time_unix = get_unix_time();
        // NOTE: On linux the frame number is sometimes 1 for the audio stream so this check doesn't make sense
        //       Instead we only update the progress if the transcode duration is greater than the old duration
        // if progress.frame != Some(0) {
        //     return;
        // }
        let Some(new_duration) = progress.total_time_transcoded.map(|t| t.to_milliseconds()) else {
            return;
        };
        if let Some(old_duration) = self.transcode_duration_milliseconds {
            if old_duration > new_duration {
                return;
            }
        };
        update_field(&mut self.transcode_size_bytes, progress.size_bytes);
        update_field(&mut self.transcode_duration_milliseconds , progress.total_time_transcoded.map(|t| t.to_milliseconds()));
        update_field(&mut self.transcode_speed_bits, progress.speed_bits);
        update_field(&mut self.transcode_speed_factor, progress.speed_factor);
    }

    pub fn update_from_source_info(&mut self, info: &ffmpeg::TranscodeSourceInfo) {
        self.end_time_unix = get_unix_time();
        // NOTE: we specify multiple sources including thumbnail which gives dodgy info
        //       we check for this by only updating from the longest duration source info
        if let Some(old_duration) = self.source_duration_milliseconds {
            if let Some(new_duration) = info.duration.map(|t| t.to_milliseconds()) {
                if new_duration < old_duration {
                    return;
                }
            }
        }
        update_field(&mut self.source_duration_milliseconds, info.duration.map(|t| t.to_milliseconds()));
        update_field(&mut self.source_start_time_milliseconds, info.start_time.map(|t| t.to_milliseconds()));
        update_field(&mut self.source_speed_bits, info.speed_bits);
    }
}

pub struct TranscodeWorker {
    key: TranscodeKey,
    state: Mutex<Box<TranscodeState>>,
    condvar: Condvar,
}

impl TranscodeWorker {
    fn new(key: TranscodeKey) -> Self {
        Self {
            key,
            state: Mutex::new(Box::new(TranscodeState::default())),
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

    pub fn get_state(&self) -> Box<TranscodeState> {
        self.state.lock().unwrap().clone()
    }
}

pub struct TranscodeWorkers {
    database: Arc<Database>,
    threadpool: Arc<AppThreadPool>,
    app_config: Arc<AppConfig>,
    download_workers: Arc<DownloadWorkers>,
    cache: Mutex<HashMap<TranscodeKey, Arc<TranscodeWorker>>>,
}

impl TranscodeWorkers {
    pub fn new(
        database: Arc<Database>,
        threadpool: Arc<AppThreadPool>,
        app_config: Arc<AppConfig>,
        download_workers: Arc<DownloadWorkers>,
    ) -> Self {
        Self {
            database,
            threadpool,
            app_config,
            download_workers,
            cache: Mutex::new(HashMap::new()),
        }
    }
    pub fn get_worker(&self, key: &TranscodeKey) -> Option<Arc<TranscodeWorker>> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).cloned()
    }

    pub fn delete_worker(&self, key: &TranscodeKey) -> Option<Arc<TranscodeWorker>> {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(key)
    }

    pub fn start_worker(&self, key: &TranscodeKey, video_info: Option<Arc<VideoItem>>) -> anyhow::Result<Arc<TranscodeWorker>> {
        // check cache hit
        let mut cache = self.cache.lock().unwrap();
        if let Some(worker) = cache.get(key) {
            let state = worker.state.lock().unwrap();
            if state.worker_status.is_healthy() {
                return Ok(worker.clone());
            }
        }
        // new cache item
        let worker = Arc::new(TranscodeWorker::new(key.clone()));
        let _old_worker = cache.insert(key.clone(), worker.clone());
        drop(cache);
        // check database item
        if let Some(db_entry) = self.database.connect()?.select_ffmpeg_entry(key)? {
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
                        log::warn!("Restarting transcode because audio file was missing: {0}", audio_path.to_string_lossy());
                    }
                }
            }
        }
        // delete database entries
        {
            let mut db_conn = self.database.connect()?;
            db_conn.delete_ffmpeg_entry(key)?;
            db_conn.insert_ffmpeg_entry(key)?;
            db_conn.select_and_update_ffmpeg_entry(key, |entry| {
                entry.status = WorkerStatus::Queued;
                entry.unix_time = get_unix_time().into();
            })?;
        }
        let inner_runner = {
            let key = key.clone();
            let database = self.database.clone();
            let app_config = self.app_config.clone();
            let download_workers = self.download_workers.clone();
            let worker = worker.clone();
            let video_info = video_info.clone();
            move || -> anyhow::Result<()> {
                let download_worker = download_workers.start_worker(&key.video_id)
                    .context(format!("Failed to start download worker while starting transcode worker: {0}", key.as_str()))?;
                download_worker.wait_busy();
                if download_worker.get_status() != WorkerStatus::Finished {
                    return Err(anyhow::anyhow!("Transcode worker failed because download worker failed: {0}", key.as_str()));
                }
                {
                    database
                        .connect()?
                        .select_and_update_ffmpeg_entry(&key, move |entry| {
                            entry.status = WorkerStatus::Running;
                        })?;
                    let mut state = worker.state.lock().unwrap();
                    state.worker_status = WorkerStatus::Running;
                    worker.condvar.notify_all();
                }
                // determine input audio path
                let input_filepath: PathBuf = {
                    let entry = database
                        .connect()?
                        .select_ytdlp_entry(&key.video_id)?;
                    let entry = entry.ok_or_else(|| anyhow::anyhow!("ytdlp row entry is missing: {0}", key.video_id.as_str()))?;
                    let audio_path = entry.audio_path.ok_or_else(|| anyhow::anyhow!("ytdlp audio path is missing: {0}", key.video_id.as_str()))?;
                    let input_filepath = app_config.data_folder.join(audio_path);
                    if !input_filepath.exists() {
                        return Err(anyhow::anyhow!("Audio file is missing: {0}", input_filepath.to_string_lossy()));
                    }
                    if !input_filepath.is_file() {
                        return Err(anyhow::anyhow!("Audio path isn't a file: {0}", input_filepath.to_string_lossy()));
                    }
                    input_filepath
                };
                // get output filepath
                let output_dirpath = format!("{0}_{1}", key.video_id.as_str(), key.audio_ext.as_str());
                let output_dirpath = app_config.transcodes_folder.join(output_dirpath);
                std::fs::create_dir_all(&output_dirpath)
                    .with_context(|| format!("Failed to create folder for transcode worker: {0}", output_dirpath.to_string_lossy()))?;

                let output_filepath = output_dirpath.join(key.as_str());
                let output_filepath = std::path::absolute(&output_filepath)
                    .with_context(|| format!("Failed to get absolute output filepath from: {0}", output_filepath.to_string_lossy()))?;
                // setup process
                let command = create_transcode_command(&key, &input_filepath, &output_filepath, video_info.as_deref(), &app_config)?;
                let mut process = Process::new(command);
                process.label = Some(format!("transcode_{0}", key.as_str()));
                process.system_log_filename = Some(output_dirpath.join("system.log"));
                process.stdout_filename = Some(output_dirpath.join("stderr.log"));
                process.stderr_filename = Some(output_dirpath.join("stdout.log"));
                process.stderr_handler = Some(Box::new({
                    let worker = worker.clone();
                    move |line: Option<&str>| {
                        let Some(line) = line else {
                            return;
                        };
                        match ffmpeg::parse_stderr_line(line) {
                            None => (),
                            Some(ffmpeg::ParsedStderrLine::TranscodeSourceInfo(ref info)) => {
                                log::debug!("[transcode] id={0} info={1:?}", worker.key.as_str(), info);
                                worker.state.lock().unwrap().update_from_source_info(info);
                            },
                            Some(ffmpeg::ParsedStderrLine::TranscodeProgress(ref progress)) => {
                                log::debug!("[transcode] id={0} progress={1:?}", worker.key.as_str(), progress);
                                worker.state.lock().unwrap().update_from_progress(progress);
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
                    .select_and_update_ffmpeg_entry(&key, {
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
                // validate output file exists
                if !output_filepath.is_file() {
                    return Err(anyhow::anyhow!("Output file is missing: {0}", output_filepath.to_string_lossy()));
                }
                database
                    .connect()?
                    .select_and_update_ffmpeg_entry(&key, move |entry| {
                        entry.audio_path = as_relative_filepath(Some(&output_filepath));
                    })?;
                Ok(())
            }
        };

        let outer_runner = {
            let key = key.clone();
            let worker = worker.clone();
            let database = self.database.clone();
            move || -> anyhow::Result<()> {
                match inner_runner() {
                    Err(err) => {
                        database
                            .connect()?
                            .select_and_update_ffmpeg_entry(&key, move |entry| {
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
                            .select_and_update_ffmpeg_entry(&key, move |entry| {
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
        self.threadpool.transcodes_worker.execute(move || {
            if let Err(err) = outer_runner() {
                log::error!("Runner failed with: {0}", err);
            }
        });
        Ok(worker)
    }
}

fn create_transcode_command(
    key: &TranscodeKey,
    input_path: &Path,
    output_path: &Path,
    video_info: Option<&VideoItem>,
    app_config: &AppConfig,
) -> anyhow::Result<Command> {
    let mut command = Command::new(&app_config.ffmpeg_command);
    let args = ffmpeg::create_ffmpeg_transcode_arguments(input_path, output_path, &key.video_id, key.audio_ext, video_info);
    command.current_dir(&app_config.current_working_directory);
    command.args(args.as_slice());
    Ok(command)
}
