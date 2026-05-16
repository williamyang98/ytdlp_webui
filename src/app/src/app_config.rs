use anyhow::Context;
use std::path::{Path, PathBuf};


#[derive(Clone,Debug)]
pub struct AppConfig {
    pub current_working_directory: PathBuf,
    pub data_folder: PathBuf,
    pub static_folder: PathBuf,
    pub downloads_folder: PathBuf,
    pub transcodes_folder: PathBuf,
    pub binaries_folder: PathBuf,
    pub database_path: PathBuf,
    pub total_transcode_threads: usize,
}

impl AppConfig {
    pub fn new(data_folder: &Path, static_folder: &Path) -> anyhow::Result<Self> {
        let get_absolute_dirpath = |dirpath: PathBuf| -> anyhow::Result<PathBuf> {
            std::path::absolute(&dirpath)
                .with_context(|| format!("Failed to get absolute dirpath from: {0}", dirpath.to_string_lossy()))
        };
        let data_folder = get_absolute_dirpath(data_folder.to_path_buf())?;
        let static_folder = get_absolute_dirpath(static_folder.to_path_buf())?;

        let downloads_folder = data_folder.join("downloads");
        let transcodes_folder = data_folder.join("transcodes");
        let binaries_folder = data_folder.join("binaries");
        let database_path = data_folder.join("index.db");
        let current_working_directory = std::env::current_dir()
            .context("Couldn't get current working directory of process")?;

        std::fs::create_dir_all(&data_folder).context("Couldn't create data folder")?;
        std::fs::create_dir_all(&downloads_folder).context("Couldn't create downloads folder")?;
        std::fs::create_dir_all(&transcodes_folder).context("Couldn't create transcodes folder")?;
        std::fs::create_dir_all(&binaries_folder).context("Couldn't create binaries folder")?;

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
            data_folder: data_folder.to_path_buf(),
            static_folder: static_folder.to_path_buf(),
            downloads_folder,
            transcodes_folder,
            binaries_folder,
            database_path,
            total_transcode_threads: 8,
        })
    }

    pub fn get_relative_data_path(&self, absolute_path: &PathBuf) -> anyhow::Result<PathBuf> {
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

    pub fn get_absolute_binary_filepath(&self, relative_filepath: &PathBuf) -> anyhow::Result<PathBuf> {
        let absolute_filepath = self.binaries_folder.join(relative_filepath);
        let absolute_filepath = std::path::absolute(&absolute_filepath)
            .with_context(|| format!("Failed to get absolute binary filepath from: {0}", absolute_filepath.to_string_lossy()))?;
        if !absolute_filepath.is_file() {
            return Err(anyhow::anyhow!("Absolute binary filepath does not exist: {0}", absolute_filepath.to_string_lossy()));
        }
        Ok(absolute_filepath)
    }

    pub fn get_absolute_data_filepath(&self, relative_filepath: &PathBuf) -> anyhow::Result<PathBuf> {
        let absolute_filepath = self.data_folder.join(relative_filepath);
        let absolute_filepath = std::path::absolute(&absolute_filepath)
            .with_context(|| format!("Failed to get absolute data filepath from: {0}", absolute_filepath.to_string_lossy()))?;
        if !absolute_filepath.is_file() {
            return Err(anyhow::anyhow!("Absolute data filepath does not exist: {0}", absolute_filepath.to_string_lossy()));
        }
        Ok(absolute_filepath)
    }
}
