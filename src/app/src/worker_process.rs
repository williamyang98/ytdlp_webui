use crate::util::ConvertCarriageReturnToNewLine;
use dyn_clone::DynClone;
use std::io::{BufReader, BufWriter, Write, Read, BufRead};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, Condvar};
use thiserror::Error;
use threadpool::ThreadPool;

#[derive(Debug,Copy,Clone)]
pub enum ProcessPipe {
    System,
    Stderr,
    Stdout,
}

#[derive(Debug,Error)]
pub enum ProcessWorkerError {
    #[error("Failed to create logging folder: {0:?}")]
    LogFolderCreateError(std::io::Error),
    #[error("Failed to create log file: {0:?}, {1:?}")]
    LogCreateError(std::io::Error, ProcessPipe),
    #[error("Failed to write to log file: {0:?}, {1:?}")]
    LogWriteFail(std::io::Error, ProcessPipe),
    #[error("Failed to acquire log file from process: {0:?}")]
    LogAcquireFail(ProcessPipe),
    #[error("Failed to start process: {0:?}")]
    ProcessStartFail(std::io::Error),
    #[error("Process ended with bad exit code: {0:?}")]
    ProcessBadExitCode(i32),
    #[error("Process failed to be killed: {0:?}")]
    ProcessKillFail(std::io::Error),
}

pub trait ProcessPipeHandler: DynClone + Send + 'static {
    fn read_line(&self, line: &str) -> ControlFlow<()>;
    fn finish(&self);
}
dyn_clone::clone_trait_object!(ProcessPipeHandler);

pub struct ProcessWorker {
    pub label: Option<String>,
    pub threadpool: Arc<ThreadPool>,
    pub logging_folder: PathBuf,
    pub stdout_filename: PathBuf,
    pub stderr_filename: PathBuf,
    pub system_log_filename: PathBuf,
    pub stdout_handler: Option<Box<dyn ProcessPipeHandler>>,
    pub stderr_handler: Option<Box<dyn ProcessPipeHandler>>,
}

impl ProcessWorker {
    pub fn new(threadpool: Arc<ThreadPool>, logging_folder: &Path) -> Self {
        let logging_folder = logging_folder.to_path_buf();
        let stdout_filename = logging_folder.join("stdout.log");
        let stderr_filename = logging_folder.join("stderr.log");
        let system_log_filename = logging_folder.join("system.log");
        Self {
            label: None,
            threadpool,
            logging_folder,
            stdout_filename,
            stderr_filename,
            system_log_filename,
            stdout_handler: None,
            stderr_handler: None,
        }
    }
}

struct ProcessPipeFence<V,E> {
    mutex: Mutex<Option<Result<V,E>>>,
    condvar: Condvar,
}

impl<V,E> Default for ProcessPipeFence<V,E> {
    fn default() -> Self {
        Self {
            mutex: Mutex::new(None),
            condvar: Condvar::new(),
        }
    }
}

impl<V,E> ProcessPipeFence<V,E> {
    fn wait(&self) -> Result<V,E> {
        let mut value = self.mutex.lock().unwrap();
        while value.is_none() {
            value = self.condvar.wait(value).unwrap();
        }
        value.take().expect("Expected result from process pipe result")
    }

    fn update(&self, new_value: Result<V,E>) {
        let mut value = self.mutex.lock().unwrap();
        *value = Some(new_value);
        self.condvar.notify_all();
    }
}

impl ProcessWorker {
    pub fn run(&self, mut command: Command) -> Result<(), ProcessWorkerError> {
        std::fs::create_dir_all(&self.logging_folder).map_err(ProcessWorkerError::LogFolderCreateError)?;

        // create system log
        let system_log_file = match std::fs::File::create(&self.system_log_filename) {
            Ok(system_log_file) => system_log_file,
            Err(err) => {
                log::error!("Failed to create system log file: path={0}, err={1:?}", self.system_log_filename.to_str().unwrap(), err);
                return Err(ProcessWorkerError::LogCreateError(err, ProcessPipe::System));
            },
        };
        let system_log_writer = Arc::new(Mutex::new(BufWriter::new(system_log_file)));
        let log_system = |args: std::fmt::Arguments| -> Result<(), ProcessWorkerError> {
            system_log_writer
                .lock().unwrap()
                .write_fmt(args)
                .map_err(|error| ProcessWorkerError::LogWriteFail(error, ProcessPipe::System))
        };

        // launch command
        let process_res = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut process = match process_res {
            Ok(process) => process,
            Err(err) => {
                log_system(format_args!("[error] process {0:?} failed to start: {1:?}", self.label, err))?;
                return Err(ProcessWorkerError::ProcessStartFail(err));
            }
        };

        // handle stdio
        let stdout_fence = {
            let pipe_type = ProcessPipe::Stdout;
            let pipe_filename = self.stdout_filename.clone();
            let pipe_reader = process.stdout.take().ok_or(ProcessWorkerError::LogAcquireFail(pipe_type))?;
            let pipe_handler = self.stdout_handler.clone();
            self.create_reader(pipe_type, pipe_filename, pipe_reader, pipe_handler)?
        };
        let stderr_fence = {
            let pipe_type = ProcessPipe::Stderr;
            let pipe_filename = self.stderr_filename.clone();
            let pipe_reader = process.stderr.take().ok_or(ProcessWorkerError::LogAcquireFail(pipe_type))?;
            let pipe_handler = self.stderr_handler.clone();
            self.create_reader(pipe_type, pipe_filename, pipe_reader, pipe_handler)?
        };

        // join stdio handles
        stdout_fence.wait()?;
        stderr_fence.wait()?;

        // shutdown process
        match process.try_wait() {
            Ok(None) => {},
            Ok(Some(exit_status)) => match exit_status.code() {
                None | Some(0) => {},
                Some(code) => {
                    log_system(format_args!("[error] {0:?} failed with bad code: {1:?}", self.label, code))?;
                    return Err(ProcessWorkerError::ProcessBadExitCode(code));
                },
            },
            Err(err) => {
                log_system(format_args!("[warn] ytdlp process failed to join: {err:?}"))?;
                if let Err(err) = process.kill() {
                    log_system(format_args!("[warn] ytdlp process failed to be killed: {err:?}"))?;
                    return Err(ProcessWorkerError::ProcessKillFail(err));
                }
            },
        }

        Ok(())
    }

    fn create_reader(
        &self,
        pipe_type: ProcessPipe,
        pipe_filename: PathBuf,
        pipe_reader: impl Read + Send + 'static,
        pipe_handler: Option<Box<dyn ProcessPipeHandler>>,
    ) -> Result<Arc<ProcessPipeFence<(), ProcessWorkerError>>, ProcessWorkerError> {
        let pipe_fence = Arc::new(ProcessPipeFence::default());
        self.threadpool.execute({
            let pipe_fence = pipe_fence.clone();
            let mut pipe_reader = BufReader::new(ConvertCarriageReturnToNewLine::new(pipe_reader));
            let pipe_file = std::fs::File::create(&pipe_filename).map_err(|e| ProcessWorkerError::LogCreateError(e, pipe_type))?;
            let mut pipe_writer = BufWriter::new(pipe_file);
            move || {
                let res = move || -> Result<(), ProcessWorkerError> {
                    let mut line = String::new();
                    loop {
                        match pipe_reader.read_line(&mut line) {
                            Err(_) => break,
                            Ok(0) => break,
                            Ok(_) => (),
                        }
                        let _ = pipe_writer.write(line.as_bytes()).map_err(|e| ProcessWorkerError::LogWriteFail(e, pipe_type))?;
                        if let Some(pipe_handler) = pipe_handler.as_ref() {
                            match pipe_handler.read_line(line.as_str()) {
                                ControlFlow::Break(()) => break,
                                ControlFlow::Continue(()) => (),
                            }
                        }
                        line.clear();
                    }
                    if let Some(pipe_handler) = pipe_handler.as_ref() {
                        pipe_handler.finish();
                    }
                    Ok(())
                } ();
                pipe_fence.update(res);
            }
        });
        Ok(pipe_fence)
    }
}
