import {
  type DownloadKey, type TranscodeKey,
  type AudioExtension,
} from "../api/ytdlp_api_schema.ts";
import { type PlaylistId, type VideoId } from "../api/youtube_api_schema.ts";
import { computed, isReactive, watch } from "vue";
import { create_youtube_link, create_youtube_playlist_link, parse_youtube_url, type YoutubeUrlParseResult } from "../utility/youtube_url.ts";
import { type CachedApiStore } from "../stores/cached_api.ts";

export interface SearchBar {
  url: string;
  audio_ext: AudioExtension;
}

export class App {
  selected_download_key: DownloadKey | null;
  selected_transcode_key: TranscodeKey | null;
  pending_request: TranscodeKey | null;
  selected_youtube_video: VideoId | null;
  selected_youtube_playlist: PlaylistId | null;
  youtube_search_bar: SearchBar;
  youtube_url_parse_result: YoutubeUrlParseResult;
  is_mounted: boolean;

  constructor() {
    this.selected_download_key = null;
    this.selected_transcode_key = null;
    this.pending_request = null;
    this.selected_youtube_video = null;
    this.selected_youtube_playlist = null;
    this.youtube_search_bar = {
      url: "",
      audio_ext: "mp3",
    };
    this.youtube_url_parse_result = {};
    this.is_mounted = false;
  }

  async get_youtube_video(cached_api: CachedApiStore, video_id: VideoId, force_refresh?: boolean) {
    await cached_api.get_youtube_video(video_id, force_refresh);
    this.selected_youtube_video = video_id;
  }

  async get_youtube_playlist(cached_api: CachedApiStore, playlist_id: PlaylistId, force_refresh?: boolean) {
    await cached_api.get_youtube_playlist(playlist_id, force_refresh);
    this.selected_youtube_playlist = playlist_id;
  }

  select_download(cached_api: CachedApiStore, key: DownloadKey) {
    this.selected_download_key = key;
    this.youtube_search_bar.url = create_youtube_link(key);
    void this.get_youtube_video(cached_api, key);
  }

  select_transcode(cached_api: CachedApiStore, key: TranscodeKey) {
    this.selected_transcode_key = key;
    this.youtube_search_bar.url = create_youtube_link(key.video_id);
    this.youtube_search_bar.audio_ext = key.audio_ext;
    void this.get_youtube_video(cached_api, key.video_id);
  }

  select_playlist_item(playlist_id: PlaylistId, video_id: VideoId) {
    this.youtube_search_bar.url = create_youtube_playlist_link(playlist_id, video_id);
  }

  on_mount(cached_api: CachedApiStore) {
    if (!isReactive(this)) {
      console.error("Tried to mount non-reactive app instance");
      return;
    }
    if (this.is_mounted) {
      console.warn("Skipping attempt to remount app instance");
      return;
    }
    this.is_mounted = true;


    const url = computed(() => this.youtube_search_bar.url);
    watch(url, (url) => {
      // clear url
      if (url.length === 0) {
        this.youtube_url_parse_result = {};
        this.selected_youtube_video = null;
        this.selected_youtube_playlist = null;
        this.pending_request = null;
        return;
      }
      // attempt to parse url
      const result = parse_youtube_url(url);
      if (result.video_id !== undefined) {
        if (result.video_id !== this.youtube_url_parse_result.video_id) {
          this.pending_request = null;
          void this.get_youtube_video(cached_api, result.video_id);
        }
      } else {
        this.selected_youtube_video = null;
        this.pending_request = null;
      }
      if (result.playlist_id !== undefined) {
        if (result.playlist_id !== this.youtube_url_parse_result.playlist_id) {
          void this.get_youtube_playlist(cached_api, result.playlist_id);
        }
      } else {
        this.selected_youtube_playlist = null;
      }
      this.youtube_url_parse_result = result;
    });
  }
}
