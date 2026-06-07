import * as api from "../api/api.ts";
import {
  type YtdlpRow, type FfmpegRow,
  type DownloadKey, type TranscodeKey,
  type TranscodeState, type DownloadState,
  type AudioExtension,
  is_worker_running,
} from "../api/ytdlp_api_schema.ts";
import { type PlaylistId, type PlaylistItem, type VideoId, type VideoItem } from "../api/youtube_api_schema.ts";
import { computed, isReactive, reactive, watch } from "vue";
import { create_youtube_link, create_youtube_playlist_link, parse_youtube_url, type YoutubeUrlParseResult } from "../utility/youtube_url.ts";

async function sleep(milliseconds: number) {
  await new Promise(resolve => setTimeout(resolve, milliseconds));
}

export class DownloadWorker {
  key: DownloadKey;
  state: DownloadState | null;
  error: string | null;
  total_requests: number;
  is_running: boolean;
  promise: Promise<void> | null;

  constructor(key: DownloadKey) {
    this.key = key;
    this.state = null;
    this.error = null;
    this.total_requests = 0;
    this.is_running = false;
    this.promise = null;
  }

  async listen(restart_if_idle?: boolean) {
    if (this.promise !== null && (this.is_running || !restart_if_idle)) {
      return this.promise;
    }
    this.is_running = true;
    this.promise = null;
    this.state = null;
    this.error = null;
    this.total_requests = 0;
    const runner = async () => {
      while (true) {
        this.total_requests += 1;
        try {
          this.state = await api.get_download_state(this.key);
        } catch (error) {
          this.error = String(error);
          break;
        }
        if (!is_worker_running(this.state.worker_status)) {
          break;
        }
        await sleep(1000);
      }
      this.is_running = false;
    }
    this.promise = runner();
    return this.promise;
  }
}

export class TranscodeWorker {
  key: TranscodeKey;
  state: TranscodeState | null;
  error: string | null;
  total_requests: number;
  is_running: boolean;
  promise: Promise<void> | null;

  constructor(key: TranscodeKey) {
    this.key = key;
    this.state = null;
    this.error = null;
    this.total_requests = 0;
    this.is_running = false;
    this.promise = null;
  }

  async listen(restart_if_idle?: boolean) {
    if (this.promise !== null && (this.is_running || !restart_if_idle)) {
      return this.promise;
    }
    this.is_running = true;
    this.promise = null;
    this.state = null;
    this.error = null;
    this.total_requests = 0;
    const runner = async () => {
      while (true) {
        this.total_requests += 1;
        try {
          this.state = await api.get_transcode_state(this.key);
        } catch (error) {
          this.error = String(error);
          break;
        }
        if (!is_worker_running(this.state.worker_status)) {
          break;
        }
        await sleep(1000);
      }
      this.is_running = false;
    }
    this.promise = runner();
    return this.promise;
  }
}

function get_transcode_worker_key(key: TranscodeKey): string {
  return `${key.video_id}_${key.audio_ext}`;
}

export interface SearchBar {
  url: string;
  audio_ext: AudioExtension;
}

export interface Playlist {
  id: PlaylistId;
  items: PlaylistItem[];
}

export class App {
  downloads: YtdlpRow[];
  transcodes: FfmpegRow[];
  selected_download_key: DownloadKey | null;
  selected_transcode_key: TranscodeKey | null;
  pending_request: TranscodeKey | null;
  youtube_video: VideoItem | null;
  youtube_playlist: Playlist | null;
  download_workers: Partial<Record<string, DownloadWorker>>;
  transcode_workers: Partial<Record<string, TranscodeWorker>>;
  youtube_search_bar: SearchBar;
  youtube_url_parse_result: YoutubeUrlParseResult;
  is_mounted: boolean;

  constructor() {
    this.downloads = [];
    this.transcodes = [];
    this.selected_download_key = null;
    this.selected_transcode_key = null;
    this.pending_request = null;
    this.youtube_video = null;
    this.youtube_playlist = null;
    this.transcode_workers = {};
    this.download_workers = {};
    this.youtube_search_bar = {
      url: "",
      audio_ext: "mp3",
    };
    this.youtube_url_parse_result = {};
    this.is_mounted = false;
  }

  async get_downloads() {
    const response = await api.get_downloads();
    this.downloads = response;
  }

  async get_transcodes() {
    const response = await api.get_transcodes();
    this.transcodes = response;
  }

  async get_download(key: DownloadKey) {
    const download = await api.get_download(key);
    const index = this.downloads.findIndex(e => e.video_id === key);
    if (index >= 0) {
      this.downloads[index] = download;
    } else {
      this.downloads.push(download);
    }
  }

  async get_transcode(key: TranscodeKey) {
    const transcode = await api.get_transcode(key);
    const index = this.transcodes.findIndex(e => e.video_id === key.video_id && e.audio_ext === key.audio_ext);
    if (index >= 0) {
      this.transcodes[index] = transcode;
    } else {
      this.transcodes.push(transcode);
    }
  }

  async get_youtube_video(video_id: VideoId, force_refresh?: boolean) {
    const video = await api.get_youtube_video(video_id, force_refresh);
    this.youtube_video = video;
  }

  async get_youtube_playlist(playlist_id: PlaylistId, force_refresh?: boolean) {
    const items = await api.get_youtube_playlist(playlist_id, force_refresh);
    this.youtube_playlist = { id: playlist_id, items };
  }

  select_download(key: DownloadKey) {
    this.selected_download_key = key;
    this.youtube_search_bar.url = create_youtube_link(key);
    const _ = this.get_youtube_video(key);
  }

  select_transcode(key: TranscodeKey) {
    this.selected_transcode_key = key;
    this.youtube_search_bar.url = create_youtube_link(key.video_id);
    this.youtube_search_bar.audio_ext = key.audio_ext;
    const _ = this.get_youtube_video(key.video_id);
  }

  select_playlist_item(playlist_id: PlaylistId, video_id: VideoId) {
    this.youtube_search_bar.url = create_youtube_playlist_link(playlist_id, video_id);
  }

  async delete_download(key: DownloadKey) {
    const _res = await api.delete_download(key);
    const index = this.downloads.findIndex(v => v.video_id === key);
    if (index >= 0) {
      this.downloads.splice(index, 1);
    }
    this.download_workers[key] = undefined;
    if (this.selected_download_key === key) {
      this.selected_download_key = null;
    }
  }

  async delete_transcode(key: TranscodeKey) {
    const _res = await api.delete_transcode(key);
    const index = this.transcodes.findIndex(v => v.video_id === key.video_id && v.audio_ext === key.audio_ext);
    if (index >= 0) {
      this.transcodes.splice(index, 1);
    }
    const record_key = get_transcode_worker_key(key);
    this.transcode_workers[record_key] = undefined;
    if (this.selected_transcode_key === key) {
      this.selected_transcode_key = null;
    }
  }

  get_download_worker(key: DownloadKey): DownloadWorker {
    let worker = this.download_workers[key];
    if (worker !== undefined) return worker;
    worker = reactive(new DownloadWorker(key));
    const _ = worker.listen();
    this.download_workers[key] = worker;
    return worker;
  }

  get_transcode_worker(key: TranscodeKey): TranscodeWorker {
    const record_key = get_transcode_worker_key(key);
    let worker = this.transcode_workers[record_key];
    if (worker !== undefined) return worker;
    worker = reactive(new TranscodeWorker(key));
    const _ = worker.listen();
    this.transcode_workers[record_key] = worker;
    return worker;
  }

  async request_transcode(key: TranscodeKey) {
    const response = await api.request_transcode(key);

    const run_download = async () => {
      await this.get_download(key.video_id);
      try {
        await this.get_download_worker(key.video_id).listen(true);
      } finally {
        await this.get_download(key.video_id);
      }
    };
    const run_transcode = async () => {
      await this.get_transcode(key);
      try {
        await this.get_transcode_worker(key).listen(true);
      } finally {
        await this.get_transcode(key);
      }
    };
    void run_download();
    void run_transcode();
    return response;
  }

  async on_mount() {
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
        this.youtube_video = null;
        this.youtube_playlist = null;
        this.pending_request = null;
        return;
      }
      // attempt to parse url
      const result = parse_youtube_url(url);
      if (result.video_id !== undefined) {
        if (result.video_id !== this.youtube_url_parse_result.video_id) {
          this.pending_request = null;
          void this.get_youtube_video(result.video_id);
        }
      } else {
        this.youtube_video = null;
        this.pending_request = null;
      }
      if (result.playlist_id !== undefined) {
        if (result.playlist_id !== this.youtube_url_parse_result.playlist_id) {
          void this.get_youtube_playlist(result.playlist_id);
        }
      } else {
        this.youtube_playlist = null;
      }
      this.youtube_url_parse_result = result;
    });

    await Promise.all([
      this.get_downloads(),
      this.get_transcodes(),
    ])
  }
}
