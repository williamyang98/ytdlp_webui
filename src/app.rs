use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

pub type WorkerThreadPool = Arc<Mutex<ThreadPool>>;

#[derive(Debug)]
pub enum WorkerError {
    SystemLogCreate(std::io::Error),
    StdoutLogCreate(std::io::Error),
    StderrLogCreate(std::io::Error),
    SystemWriteFail(std::io::Error),
    StdoutWriteFail(std::io::Error),
    StderrWriteFail(std::io::Error),
    StdoutMissing,
    StderrMissing,
    StdoutThreadJoin(Box<dyn std::any::Any + Send + 'static>),
    StderrThreadJoin(Box<dyn std::any::Any + Send + 'static>),
    DatabaseConnection(r2d2::Error),
    DatabaseExecute(rusqlite::Error),
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
