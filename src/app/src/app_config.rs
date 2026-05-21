use anyhow::Context;
use std::path::{Path, PathBuf};

#[derive(Clone,Debug)]
pub struct AppConfig {
    pub current_working_directory: PathBuf,
    pub data_folder: PathBuf,
    pub static_folder: PathBuf,
    pub downloads_folder: PathBuf,
    pub transcodes_folder: PathBuf,
    pub playlists_folder: PathBuf,
    pub youtube_api_cache_folder: PathBuf,
    pub database_path: PathBuf,
    pub total_transcode_threads: usize,
    pub ffmpeg_command: PathBuf,
    pub ytdlp_command: PathBuf,
}

fn get_command_from_environment_variable(key: &'static str) -> anyhow::Result<PathBuf> {
    let command = std::env::var(key).context(format!("Missing {0}", key))?;
    if command.is_empty() {
        return Err(anyhow::anyhow!("Got empty command string for environment variable {0}", key));
    }
    let command = which::which(&command)
        .with_context(|| format!("Failed to find binary path for {0}: {1}", key, &command))?;
    Ok(command)
}

impl AppConfig {
    pub fn new(env_file: &Path, data_folder: &Path, static_folder: &Path) -> anyhow::Result<Self> {
        dotenvy::from_path(env_file)
            .with_context(|| anyhow::anyhow!("Failed to read dotenv file at: {0}", env_file.to_string_lossy()))?;

        let ffmpeg_command = get_command_from_environment_variable("FFMPEG_BIN")?;
        log::debug!("Got ffmpeg command: {0}", &ffmpeg_command.to_string_lossy());
        let ytdlp_command = get_command_from_environment_variable("YTDLP_BIN")?;
        log::debug!("Got ytdlp command: {0}", &ytdlp_command.to_string_lossy());

        let data_folder = data_folder.to_path_buf();
        let static_folder = static_folder.to_path_buf();

        let get_absolute_dirpath = |dirpath: &Path| -> anyhow::Result<PathBuf> {
            std::path::absolute(dirpath)
                .with_context(|| format!("Failed to get absolute dirpath from: {0}", dirpath.to_string_lossy()))
        };
        let data_folder = get_absolute_dirpath(&data_folder.to_path_buf())?;
        let static_folder = get_absolute_dirpath(&static_folder.to_path_buf())?;

        let downloads_folder = data_folder.join("downloads");
        let transcodes_folder = data_folder.join("transcodes");
        let playlists_folder = data_folder.join("playlists");
        let youtube_api_cache_folder = data_folder.join("youtube_api_cache");
        let database_path = data_folder.join("index.db");
        let current_working_directory = std::env::current_dir()
            .context("Couldn't get current working directory of process")?;

        let create_folder = |folder: &Path| -> anyhow::Result<()> {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("Couldn't create folder: {0}", folder.to_string_lossy()))
        };

        create_folder(&data_folder)?;
        create_folder(&downloads_folder)?;
        create_folder(&transcodes_folder)?;
        create_folder(&playlists_folder)?;
        create_folder(&youtube_api_cache_folder)?;

        if !static_folder.exists() {
            return Err(anyhow::anyhow!("Static dirpath doesn't exist: {0}", static_folder.to_string_lossy()));
        }
        if !static_folder.is_dir() {
            return Err(anyhow::anyhow!("Static dirpath isn't a directory: {0}", static_folder.to_string_lossy()));
        }
        if data_folder.exists() && !data_folder.is_dir() {
            return Err(anyhow::anyhow!("Data dirpath isn't a directory exist: {0}", data_folder.to_string_lossy()));
        }

        Ok(Self {
            current_working_directory,
            data_folder,
            static_folder,
            downloads_folder,
            transcodes_folder,
            playlists_folder,
            youtube_api_cache_folder,
            database_path,
            ffmpeg_command,
            ytdlp_command,
            total_transcode_threads: 8,
        })
    }

    pub fn get_relative_data_path(&self, absolute_path: &Path) -> anyhow::Result<PathBuf> {
        let relative_path = match absolute_path.strip_prefix(&self.data_folder) {
            Ok(path) => path,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to get relative path from absolute path ({0}) relative to data folder ({1}): {2:?}",
                    absolute_path.to_string_lossy(), &self.data_folder.to_string_lossy(), err,
                ));
            },
        };
        Ok(relative_path.to_path_buf())
    }

    pub fn get_absolute_data_path(&self, relative_path: &Path) -> anyhow::Result<PathBuf> {
        let absolute_path = self.data_folder.join(relative_path);
        let absolute_path = std::path::absolute(&absolute_path)
            .with_context(|| format!("Failed to get absolute data path from: {0}", absolute_path.to_string_lossy()))?;
        if !absolute_path.exists() {
            return Err(anyhow::anyhow!("Absolute data filepath does not exist: {0}", absolute_path.to_string_lossy()));
        }
        Ok(absolute_path)
    }

    pub fn check_if_path_whitelisted_for_delete(&self, absolute_path: &Path) -> bool {
        if absolute_path.starts_with(&self.downloads_folder) { return true; }
        if absolute_path.starts_with(&self.transcodes_folder) { return true; }
        if absolute_path.starts_with(&self.playlists_folder) { return true; }
        if absolute_path.starts_with(&self.youtube_api_cache_folder) { return true; }
        false
    }

    pub fn delete_file(&self, relative_path: &Path) -> Result<(), std::io::Error> {
        let absolute_path = self.data_folder.join(relative_path);
        if !self.check_if_path_whitelisted_for_delete(&absolute_path) {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
        std::fs::remove_file(absolute_path)
    }

    pub fn delete_folder(&self, relative_path: &Path) -> Result<(), std::io::Error> {
        let absolute_path = self.data_folder.join(relative_path);
        if !self.check_if_path_whitelisted_for_delete(&absolute_path) {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
        std::fs::remove_dir(absolute_path)
    }
}
