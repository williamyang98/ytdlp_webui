use dashmap::DashMap;
use youtube_api::{Api, VideoItem, VideoId, PlaylistItem, PlaylistId, PaginatedResponse};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::Context;
use crate::app_config::AppConfig;

pub struct YoutubeApiCache {
    api: Api,
    videos_cache: DashMap<VideoId, Arc<PaginatedResponse<VideoItem>>>,
    playlists_cache: DashMap<PlaylistId, Arc<PaginatedResponse<PlaylistItem>>>,
    videos_folder: PathBuf,
    playlists_folder: PathBuf,
}

impl YoutubeApiCache {
    pub fn new(app_config: Arc<AppConfig>) -> anyhow::Result<Self> {
        let create_folder = |folder: &Path| -> anyhow::Result<()> {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("Couldn't create folder: {0}", folder.to_string_lossy()))
        };

        let videos_folder = app_config.youtube_api_cache_folder.join("videos");
        let playlists_folder = app_config.youtube_api_cache_folder.join("playlists");
        create_folder(&videos_folder)?;
        create_folder(&playlists_folder)?;

        let videos_cache = DashMap::new();
        let playlists_cache = DashMap::new();
        let api = Api::default();
        Ok(Self {
            api,
            videos_cache,
            playlists_cache,
            videos_folder,
            playlists_folder,
        })
    }

    pub async fn get_video(&self, video_id: &VideoId) -> anyhow::Result<Arc<PaginatedResponse<VideoItem>>> {
        if let Some(response) = self.videos_cache.get(video_id) {
            return Ok(response.clone());
        }

        // load from filepath
        let filepath = self.videos_folder.join(format!("{0}.json", video_id.as_str()));
        let load_from_filepath = |path: &Path| -> anyhow::Result<Option<PaginatedResponse<VideoItem>>> {
            if !path.exists() {
                return Ok(None);
            }
            let data = std::fs::read_to_string(path)?;
            let response: PaginatedResponse<VideoItem> = serde_json::from_str(data.as_str())?;
            Ok(Some(response))
        };

        let video: Option<PaginatedResponse<VideoItem>> = match load_from_filepath(&filepath) {
            Ok(None) => {
                log::debug!("video cache miss from disk: {0}", filepath.to_string_lossy());
                None
            },
            Ok(Some(response)) => {
                log::debug!("got video cache hit from disk: {0}", filepath.to_string_lossy());
                Some(response)
            },
            Err(err) => {
                log::error!("failed to parse cached video on disk at {0}: {1:?}", filepath.to_string_lossy(), err);
                None
            },
        };

        let write_to_filepath = |path: &Path, text: &str| -> anyhow::Result<()> {
            let mut file = std::fs::File::create(path)?;
            file.write_all(text.as_bytes())?;
            Ok(())
        };

        let video = match video {
            Some(video) => video,
            None => {
                let response = self.api.get_video(video_id).await?;
                if let Err(err) = write_to_filepath(&filepath, &response.text) {
                    log::error!("failed to cache video to disk at {0}: {1:?}", filepath.to_string_lossy(), err);
                } else {
                    log::debug!("successfully cached video to disk at {0}", filepath.to_string_lossy());
                }
                response.value
            },
        };
        let video = Arc::new(video);
        self.videos_cache.insert(video_id.clone(), video.clone());
        Ok(video)
    }

    pub async fn get_playlist(&self, playlist_id: &PlaylistId) -> anyhow::Result<Arc<PaginatedResponse<PlaylistItem>>> {
        if let Some(response) = self.playlists_cache.get(playlist_id) {
            return Ok(response.clone());
        }

        // load from filepath
        let filepath = self.playlists_folder.join(format!("{0}.json", playlist_id.as_str()));
        let load_from_filepath = |path: &Path| -> anyhow::Result<Option<PaginatedResponse<PlaylistItem>>> {
            if !path.exists() {
                return Ok(None);
            }
            let data = std::fs::read_to_string(path)?;
            let response: PaginatedResponse<PlaylistItem> = serde_json::from_str(data.as_str())?;
            Ok(Some(response))
        };

        let playlist: Option<PaginatedResponse<PlaylistItem>> = match load_from_filepath(&filepath) {
            Ok(None) => {
                log::debug!("playlist cache miss from disk: {0}", filepath.to_string_lossy());
                None
            },
            Ok(Some(response)) => {
                log::debug!("got playlist cache hit from disk: {0}", filepath.to_string_lossy());
                Some(response)
            },
            Err(err) => {
                log::error!("failed to parse cached playlist on disk at {0}: {1:?}", filepath.to_string_lossy(), err);
                None
            },
        };

        let write_to_filepath = |path: &Path, text: &str| -> anyhow::Result<()> {
            let mut file = std::fs::File::create(path)?;
            file.write_all(text.as_bytes())?;
            Ok(())
        };

        let playlist = match playlist {
            Some(playlist) => playlist,
            None => {
                let response = self.api.get_playlist(playlist_id).await?;
                if let Err(err) = write_to_filepath(&filepath, &response.text) {
                    log::error!("failed to cache playlist to disk at {0}: {1:?}", filepath.to_string_lossy(), err);
                } else {
                    log::debug!("successfully cached playlist to disk at {0}", filepath.to_string_lossy());
                }
                response.value
            },
        };
        let playlist = Arc::new(playlist);
        self.playlists_cache.insert(playlist_id.clone(), playlist.clone());
        Ok(playlist)
    }
}
