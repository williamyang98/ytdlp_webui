import {
  type DownloadKey, type TranscodeKey,
  type AudioExtension,
} from "../api/ytdlp_api_schema.ts";
import { type PlaylistId, type VideoId } from "../api/youtube_api_schema.ts";
import { ref, computed, watch } from "vue";
import { create_youtube_link, create_youtube_playlist_link, parse_youtube_url, type YoutubeUrlParseResult } from "../utility/youtube_url.ts";
import { get_transcode_key_hash, use_cached_api_store } from "../stores/cached_api.ts";
import { defineStore } from "pinia";

export interface SearchBar {
  url: string;
  audio_ext: AudioExtension;
}

export const use_shared_app_store = defineStore("shared_app", () => {
  const selected_download_key = ref<DownloadKey | null>(null);
  const selected_transcode_key = ref<TranscodeKey | null>(null);
  const pending_request = ref<TranscodeKey | null>(null);
  const selected_youtube_video = ref<VideoId | null>(null);
  const selected_youtube_playlist = ref<PlaylistId | null>(null);
  const youtube_search_bar = ref<SearchBar>({
    url: "",
    audio_ext: "mp3",
  });
  const youtube_url_parse_result = ref<YoutubeUrlParseResult>({});

  const cached_api = use_cached_api_store();

  function select_youtube_video(video_id: VideoId, force_refresh?: boolean) {
    void cached_api.get_youtube_video(video_id, force_refresh);
    selected_youtube_video.value = video_id;
  }

  function select_youtube_playlist(playlist_id: PlaylistId, force_refresh?: boolean) {
    void cached_api.get_youtube_playlist(playlist_id, force_refresh);
    selected_youtube_playlist.value = playlist_id;
  }

  function select_download(key: DownloadKey) {
    selected_download_key.value = key;
    youtube_search_bar.value.url = create_youtube_link(key);
    select_youtube_video(key);
  }

  function select_transcode(key: TranscodeKey) {
    selected_transcode_key.value = key;
    youtube_search_bar.value.url = create_youtube_link(key.video_id);
    youtube_search_bar.value.audio_ext = key.audio_ext;
    select_youtube_video(key.video_id);
  }

  function select_youtube_playlist_item(playlist_id: PlaylistId, video_id: VideoId) {
    youtube_search_bar.value.url = create_youtube_playlist_link(playlist_id, video_id);
    select_youtube_playlist(playlist_id);
    select_youtube_video(video_id);
  }

  // attempt to see if there is a pre-existing finished or ongoing download and focus on it
  function try_focus_pending_request(key: TranscodeKey): boolean {
    const download = cached_api.downloads[key.video_id];
    const hash = get_transcode_key_hash(key);
    const transcode = cached_api.transcodes[hash];
    if (download === undefined && transcode === undefined) {
      return false;
    }
    pending_request.value = key;
    return true;
  }

  const url = computed(() => youtube_search_bar.value.url);
  watch(url, (url) => {
    // clear url
    if (url.length === 0) {
      youtube_url_parse_result.value = {};
      selected_youtube_video.value = null;
      selected_youtube_playlist.value = null;
      pending_request.value = null;
      return;
    }
    // attempt to parse url
    const result = parse_youtube_url(url);
    if (result.video_id !== undefined) {
      if (result.video_id !== youtube_url_parse_result.value.video_id) {
        select_youtube_video(result.video_id);
        const transcode_key: TranscodeKey = {
          video_id: result.video_id,
          audio_ext: youtube_search_bar.value.audio_ext,
        };
        if (!try_focus_pending_request(transcode_key)) {
          pending_request.value = null;
        }
      }
    } else {
      selected_youtube_video.value = null;
      pending_request.value = null;
    }
    if (result.playlist_id !== undefined) {
      if (result.playlist_id !== youtube_url_parse_result.value.playlist_id) {
        select_youtube_playlist(result.playlist_id);
      }
    } else {
      selected_youtube_playlist.value = null;
    }
    youtube_url_parse_result.value = result;
  });

  return {
    // state
    selected_download_key,
    selected_transcode_key,
    pending_request,
    selected_youtube_video,
    selected_youtube_playlist,
    youtube_search_bar,
    youtube_url_parse_result,
    // actions
    select_youtube_video,
    select_youtube_playlist,
    select_youtube_playlist_item,
    select_download,
    select_transcode,
    try_focus_pending_request,
  };
});

export type SharedAppStore = ReturnType<typeof use_shared_app_store>;
