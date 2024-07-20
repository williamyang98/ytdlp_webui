use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Condvar};
use thiserror::Error;
use threadpool::ThreadPool;

pub type WorkerThreadPool = Arc<Mutex<ThreadPool>>;
pub type WorkerCacheEntry<T> = Arc<(Mutex<T>, Condvar)>;

#[derive(Debug,Error)]
pub enum WorkerError {
    #[error("Failed to create system log: {0:?}")]
    SystemLogCreate(std::io::Error),
    #[error("Failed to create stdout log: {0:?}")]
    StdoutLogCreate(std::io::Error),
    #[error("Failed to create stderr log: {0:?}")]
    StderrLogCreate(std::io::Error),
    #[error("Failed to write to system log: {0:?}")]
    SystemWriteFail(std::io::Error),
    #[error("Failed to write to stdout log: {0:?}")]
    StdoutWriteFail(std::io::Error),
    #[error("Failed to write to stderr log: {0:?}")]
    StderrWriteFail(std::io::Error),
    #[error("Failed to acquire stdout from process")]
    StdoutMissing,
    #[error("Failed to acquire stderr from process")]
    StderrMissing,
    #[error("Failed to join stdout thread: {0:?}")]
    StdoutThreadJoin(Box<dyn std::any::Any + Send + 'static>),
    #[error("Failed to join stderr thread: {0:?}")]
    StderrThreadJoin(Box<dyn std::any::Any + Send + 'static>),
}

#[derive(Clone,Debug)]
pub struct AppConfig {
    pub root: PathBuf,
    pub data: PathBuf,
    pub download: PathBuf,
    pub transcode: PathBuf,
    pub ffmpeg_binary: PathBuf,
    pub ytdlp_binary: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let root = Path::new(".");
        let data = root.join("data");
        Self {
            root: root.to_owned(),
            data: data.to_owned(), 
            download: data.join("downloads"),
            transcode: data.join("transcode"),
            ffmpeg_binary: root.join("bin").join("ffmpeg.exe"),
            ytdlp_binary: root.join("bin").join("yt-dlp.exe"),
        }
    }
}

impl AppConfig {
    pub fn seed_directories(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.data)?;
        std::fs::create_dir_all(&self.download)?;
        std::fs::create_dir_all(&self.transcode)?;
        Ok(())
    }
}
