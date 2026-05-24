use dashmap::DashMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use youtube_api::{Api, VideoItem, VideoId, PlaylistItem, PlaylistId};
use std::hash::Hash;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::Context;
use crate::app_config::AppConfig;

pub struct YoutubeApiCache {
    api: Api,
    videos_cache: DashMap<VideoId, Arc<VideoItem>>,
    playlists_cache: DashMap<PlaylistId, Arc<Vec<PlaylistItem>>>,
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

    pub async fn get_video(&self, video_id: &VideoId, force_refresh: bool) -> anyhow::Result<Arc<VideoItem>> {
        let filepath = self.videos_folder.join(format!("{0}.json", video_id.as_str()));
        self.cache_response(
            "video",
            &filepath,
            video_id,
            force_refresh,
            &self.videos_cache,
            async || self.api.get_video(video_id).await,
        ).await
    }

    pub async fn get_playlist(&self, playlist_id: &PlaylistId, force_refresh: bool) -> anyhow::Result<Arc<Vec<PlaylistItem>>> {
        let filepath = self.playlists_folder.join(format!("{0}.json", playlist_id.as_str()));
        self.cache_response(
            "playlist",
            &filepath,
            playlist_id,
            force_refresh,
            &self.playlists_cache,
            async || self.api.get_playlist(playlist_id).await,
        ).await
    }

    async fn cache_response<K,V>(
        &self,
        label: &'static str,
        filepath: &Path,
        key: &K,
        force_refresh: bool,
        cache: &DashMap<K, Arc<V>>,
        api_fallback: impl AsyncFnOnce() -> anyhow::Result<V>,
    ) -> anyhow::Result<Arc<V>>
    where
        K: Hash + Eq + Clone,
        V: DeserializeOwned + Serialize,
    {
        if let Some(response) = cache.get(key) {
            return Ok(response.clone());
        }

        // load from filepath
        let load_from_filepath = |path: &Path| -> anyhow::Result<Option<V>> {
            if !path.exists() {
                return Ok(None);
            }
            if force_refresh {
                log::debug!("{label} force refresh cache ignoring existing file on disk: {0}", path.to_string_lossy());
                return Ok(None);
            }
            let data = std::fs::read_to_string(path)?;
            let response: V = serde_json::from_str(data.as_str())?;
            Ok(Some(response))
        };

        let value: Option<V> = match load_from_filepath(filepath) {
            Ok(None) => {
                log::debug!("{label} cache miss from disk: {0}", filepath.to_string_lossy());
                None
            },
            Ok(Some(response)) => {
                log::debug!("got {label} cache hit from disk: {0}", filepath.to_string_lossy());
                Some(response)
            },
            Err(err) => {
                log::error!("failed to parse cached {label} on disk at {0}: {1:?}", filepath.to_string_lossy(), err);
                None
            },
        };

        let write_to_filepath = |path: &Path, value: &V| -> anyhow::Result<()> {
            let mut file = std::fs::File::create(path)?;
            let text = serde_json::to_string(value)?;
            file.write_all(text.as_bytes())?;
            Ok(())
        };

        let value = match value {
            Some(value) => value,
            None => {
                // store entire response body onto disk, not just reserialising the deserialised value
                let value = api_fallback().await?;
                if let Err(err) = write_to_filepath(filepath, &value) {
                    log::error!("failed to cache {label} to disk at {0}: {1:?}", filepath.to_string_lossy(), err);
                } else {
                    log::debug!("successfully cached {label} to disk at {0}", filepath.to_string_lossy());
                }
                value
            },
        };
        let value = Arc::new(value);
        cache.insert(key.clone(), value.clone());
        Ok(value)
    }
}
