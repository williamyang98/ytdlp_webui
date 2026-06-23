use crate::util::ConvertCarriageReturnToNewLine;
use std::any::Any;
use std::io::{BufReader, BufWriter, Write, Read, BufRead};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use thiserror::Error;

#[derive(Debug,Copy,Clone)]
pub enum ProcessPipe {
    System,
    Stderr,
    Stdout,
}

#[derive(Debug,Error)]
pub enum ProcessError {
    #[error("Failed to create log file: {0:?}, {1:?}")]
    LogCreateError(std::io::Error, ProcessPipe),
    #[error("Failed to write to log file: {0:?}, {1:?}")]
    LogWriteFail(std::io::Error, ProcessPipe),
    #[error("Failed to join thread for {1:?}: {0:?}")]
    ThreadJoinFail(Box<dyn Any + Send>, ProcessPipe),
    #[error("Failed to acquire log file from process: {0:?}")]
    LogAcquireFail(ProcessPipe),
    #[error("Failed to start process: {0:?}")]
    ProcessStartFail(std::io::Error),
    #[error("Process ended with bad exit code: {0:?}")]
    ProcessBadExitCode(i32),
    #[error("Process failed to be killed: {0:?}")]
    ProcessKillFail(std::io::Error),
}

pub type ProcessPipeHandler = Box<dyn FnMut(Option<&str>) + Send + 'static>;

pub struct Process {
    command: Command,
    pub label: Option<String>,
    pub stdout_filename: Option<PathBuf>,
    pub stderr_filename: Option<PathBuf>,
    pub system_log_filename: Option<PathBuf>,
    pub stdout_handler: Option<ProcessPipeHandler>,
    pub stderr_handler: Option<ProcessPipeHandler>,
}

impl Process {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            label: None,
            stdout_filename: None,
            stderr_filename: None,
            system_log_filename: None,
            stdout_handler: None,
            stderr_handler: None,
        }
    }

    pub fn run(mut self) -> Result<(), ProcessError> {
        // create system log
        let mut system_log_writer = match self.system_log_filename.as_ref() {
            None => Ok(None),
            Some(filename) => match std::fs::File::create(filename) {
                Ok(file) => Ok(Some(BufWriter::new(file))),
                Err(err) => Err(ProcessError::LogCreateError(err, ProcessPipe::System)),
            },
        }?;

        let mut log_system = |args: std::fmt::Arguments| -> Result<(), ProcessError> {
            if let Some(writer) = system_log_writer.as_mut() {
                writer.write_fmt(args).map_err(|error| ProcessError::LogWriteFail(error, ProcessPipe::System))
            } else {
                Ok(())
            }
        };

        // launch command
        let process_res = self.command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut process = match process_res {
            Ok(process) => process,
            Err(err) => {
                log_system(format_args!("[error] process {0:?} failed to start: {1:?}", self.label, err))?;
                return Err(ProcessError::ProcessStartFail(err));
            }
        };

        // handle stdio
        let stdout_thread = {
            let pipe_type = ProcessPipe::Stdout;
            let pipe_filename = self.stdout_filename.clone();
            let pipe_reader = process.stdout.take().ok_or(ProcessError::LogAcquireFail(pipe_type))?;
            let pipe_handler = self.stdout_handler.take();
            self.create_reader(pipe_type, pipe_filename, pipe_reader, pipe_handler)
        }?;
        let stderr_thread = {
            let pipe_type = ProcessPipe::Stderr;
            let pipe_filename = self.stderr_filename.clone();
            let pipe_reader = process.stderr.take().ok_or(ProcessError::LogAcquireFail(pipe_type))?;
            let pipe_handler = self.stderr_handler.take();
            self.create_reader(pipe_type, pipe_filename, pipe_reader, pipe_handler)
        }?;

        // join stdio handles
        let stderr_res = stderr_thread.join().map_err(|e| ProcessError::ThreadJoinFail(e, ProcessPipe::Stderr))?;
        let stdout_res = stdout_thread.join().map_err(|e| ProcessError::ThreadJoinFail(e, ProcessPipe::Stdout))?;
        stderr_res?;
        stdout_res?;

        // shutdown process
        match process.try_wait() {
            Ok(None) => {},
            Ok(Some(exit_status)) => match exit_status.code() {
                None | Some(0) => {},
                Some(code) => {
                    log_system(format_args!("[error] {0:?} failed with bad code: {1:?}", self.label, code))?;
                    return Err(ProcessError::ProcessBadExitCode(code));
                },
            },
            Err(err) => {
                log_system(format_args!("[warn] {0:?} process failed to join: {err:?}", self.label))?;
                if let Err(err) = process.kill() {
                    log_system(format_args!("[warn] {0:?} process failed to be killed: {err:?}", self.label))?;
                    return Err(ProcessError::ProcessKillFail(err));
                }
            },
        }

        Ok(())
    }

    fn create_reader(
        &self,
        pipe_type: ProcessPipe,
        pipe_filename: Option<PathBuf>,
        pipe_reader: impl Read + Send + 'static,
        mut pipe_handler: Option<ProcessPipeHandler>,
    ) -> Result<JoinHandle<Result<(), ProcessError>>, ProcessError> {
        let mut pipe_writer = match pipe_filename.as_ref() {
            None => Ok(None),
            Some(filename) => match std::fs::File::create(filename) {
                Ok(file) => Ok(Some(BufWriter::new(file))),
                Err(err) => Err(ProcessError::LogCreateError(err, pipe_type)),
            },
        }?;
        let mut pipe_reader = BufReader::new(ConvertCarriageReturnToNewLine::new(pipe_reader));

        let handle = std::thread::spawn(move || {
            let mut line = String::new();
            loop {
                match pipe_reader.read_line(&mut line) {
                    Err(_) => break,
                    Ok(0) => break,
                    Ok(_) => (),
                }
                if let Some(pipe_writer) = pipe_writer.as_mut() {
                    pipe_writer.write(line.as_bytes()).map_err(|e| ProcessError::LogWriteFail(e, pipe_type))?;
                }
                if let Some(pipe_handler) = pipe_handler.as_mut() {
                    pipe_handler(Some(line.as_str()));
                }
                line.clear();
            }
            if let Some(pipe_handler) = pipe_handler.as_mut() {
                pipe_handler(None);
            }
            Ok(())
        });
        Ok(handle)
    }
}
