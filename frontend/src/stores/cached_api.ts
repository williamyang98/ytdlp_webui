import { defineStore } from "pinia";
import * as api from "../api/api.ts";
import type {
  VideoId,
  PlaylistId,
  VideoItem,
  PlaylistItem,
} from "../api/youtube_api_schema.ts";
import type {
  DownloadKey, TranscodeKey,
  DownloadState, TranscodeState,
  FfmpegRow, YtdlpRow,
} from "../api/ytdlp_api_schema.ts";
import { type Ref, ref } from "vue";
import { is_worker_running } from "../api/ytdlp_api_schema.ts";

export function get_transcode_key_hash(key: TranscodeKey) {
  return `${key.video_id}_${key.audio_ext}`;
}

async function sleep(milliseconds: number) {
  await new Promise(resolve => setTimeout(resolve, milliseconds));
}

export interface BackgroundWorkerCreator<K,V> {
  key: K;
  get_value: (key: K) => Promise<V>;
  set_value: (value: V) => void;
  is_finished: () => boolean;
}

export class BackgroundWorker<K,V> {
  key: K;
  total_requests: number;
  is_running: boolean;
  get_value: (key: K) => Promise<V>;
  set_value: (value: V) => void;
  is_finished: () => boolean;
  error: string | null;
  promise: Promise<void> | null;

  constructor({ key, get_value, set_value, is_finished }: BackgroundWorkerCreator<K,V>) {
    this.key = key;
    this.get_value = get_value;
    this.set_value = set_value;
    this.is_finished = is_finished;
    this.total_requests = 0;
    this.is_running = false;
    this.error = null;
    this.promise = null;
  }

  start(restart_if_idle?: boolean) {
    if (this.promise !== null && (this.is_running || !restart_if_idle)) {
      return false;
    }
    this.is_running = true;
    this.promise = null;
    this.error = null;
    this.total_requests = 0;
    this.promise = null;

    const runner = async () => {
      while (true) {
        this.total_requests += 1;
        try {
          const value = await this.get_value(this.key);
          this.set_value(value);
          if (this.is_finished()) {
            break;
          }
        } catch (error) {
          this.error = String(error);
          console.error(error);
          break;
        }
        await sleep(1000);
      }
      this.is_running = false;
    }
    this.promise = runner();
    return true;
  }
}

export const use_cached_api_store = defineStore("cached_api", () => {
  type Cache<V> = Ref<Partial<Record<string, V>>>;
  const transcodes: Cache<FfmpegRow> = ref({});
  const downloads: Cache<YtdlpRow> = ref({});
  const transcode_state: Cache<TranscodeState> = ref({});
  const download_state: Cache<DownloadState> = ref({});
  const transcode_background_worker: Cache<BackgroundWorker<TranscodeKey, TranscodeState>> = ref({});
  const download_background_worker: Cache<BackgroundWorker<DownloadKey, DownloadState>> = ref({});
  const youtube_videos: Cache<VideoItem> = ref({});
  const youtube_playlists: Cache<PlaylistItem[]> = ref({});
  const youtube_videos_error: Cache<string> = ref({});
  const youtube_playlists_error: Cache<string> = ref({});
  const youtube_videos_promise: Cache<Promise<VideoItem>> = ref({});
  const youtube_playlists_promise: Cache<Promise<PlaylistItem[]>> = ref({});
  const ytdlp_version = ref<string | null>(null);
  const ytdlp_update_log = ref<string | null>(null);
  const ytdlp_version_promise: Ref<Promise<string> | null> = ref(null);
  const ytdlp_update_promise: Ref<Promise<string> | null> = ref(null);


  async function get_transcodes(force?: boolean) {
    if (force !== true) {
      return Object.values(transcodes.value).filter(x => x !== undefined);
    }
    const new_transcodes = await api.get_transcodes();
    transcodes.value = Object.fromEntries(new_transcodes.map(v => [get_transcode_key_hash(v), v]));
    return transcodes;
  }

  async function get_transcode(key: TranscodeKey, force?: boolean) {
    const hash = get_transcode_key_hash(key);
    const old_transcode = transcodes.value[hash];
    if (force !== true && old_transcode !== undefined) {
      return old_transcode;
    }
    const new_transcode = await api.get_transcode(key);
    transcodes.value[hash] = new_transcode;
    return new_transcode;
  }

  async function delete_transcode(key: TranscodeKey) {
    const response = await api.delete_transcode(key);
    const hash = get_transcode_key_hash(key);
    delete transcodes.value[hash]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
    delete transcode_state.value[hash]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
    delete transcode_background_worker.value[hash]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
    return response;
  }

  async function get_downloads(force?: boolean) {
    if (force !== true) {
      return Object.values(downloads.value).filter(x => x !== undefined);
    }
    const new_downloads = await api.get_downloads();
    downloads.value = Object.fromEntries(new_downloads.map(v => [v.video_id, v]));
    return downloads;
  }

  async function get_download(key: DownloadKey, force?: boolean) {
    const old_download = downloads.value[key];
    if (force !== true && old_download !== undefined) {
      return old_download;
    }
    const new_download = await api.get_download(key);
    downloads.value[key] = new_download;
    return new_download;
  }

  async function delete_download(key: DownloadKey) {
    const response = await api.delete_download(key);
    delete downloads.value[key]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
    delete download_state.value[key]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
    delete download_background_worker.value[key]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
    return response;
  }

  async function get_youtube_video(video_id: VideoId, force?: boolean) {
    const old_promise = youtube_videos_promise.value[video_id];
    if (force !== true && old_promise !== undefined) {
      return await old_promise;
    }
    const new_promise_runner = async () => {
      const old_value = youtube_videos.value[video_id];
      const old_error = youtube_videos_error.value[video_id];
      if (force !== true) {
        if (old_value !== undefined) {
          return old_value;
        }
        if (old_error !== undefined) {
          throw new Error(old_error);
        }
      }

      try {
        const new_value = await api.get_youtube_video(video_id, force);
        youtube_videos.value[video_id] = new_value;
        if (old_error !== undefined) {
          delete youtube_videos_error.value[video_id]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
        }
        return new_value;
      } catch (error: unknown) {
        youtube_videos_error.value[video_id] = String(error);
        throw error;
      }
    };
    const new_promise = new_promise_runner();
    youtube_videos_promise.value[video_id] = new_promise;
    return await new_promise;
  }

  async function get_youtube_playlist(playlist_id: PlaylistId, force?: boolean) {
    const old_promise = youtube_playlists_promise.value[playlist_id];
    if (force !== true && old_promise !== undefined) {
      return await old_promise;
    }
    const new_promise_runner = async () => {
      const old_value = youtube_playlists.value[playlist_id];
      const old_error = youtube_playlists_error.value[playlist_id];
      if (force !== true) {
        if (old_value !== undefined) {
          return old_value;
        }
        if (old_error !== undefined) {
          throw new Error(old_error);
        }
      }

      try {
        const new_value = await api.get_youtube_playlist(playlist_id, force);
        youtube_playlists.value[playlist_id] = new_value;
        if (old_error !== undefined) {
          delete youtube_playlists_error.value[playlist_id]; // eslint-disable-line @typescript-eslint/no-dynamic-delete
        }
        return new_value;
      } catch (error: unknown) {
        youtube_playlists_error.value[playlist_id] = String(error);
        throw error;
      }
    };
    const new_promise = new_promise_runner();
    youtube_playlists_promise.value[playlist_id] = new_promise;
    return await new_promise;
  }

  function start_transcode_background_worker(key: TranscodeKey, restart_if_idle?: boolean) {
    const hash = get_transcode_key_hash(key);
    let worker = transcode_background_worker.value[hash];
    if (worker === undefined) {
      worker = new BackgroundWorker<TranscodeKey, TranscodeState>({
        key,
        get_value: (key: TranscodeKey) => api.get_transcode_state(key),
        set_value: (new_value: TranscodeState) => {
          const old_value = transcode_state.value[hash];
          if (old_value?.worker_status !== new_value.worker_status) {
            void get_transcode(key, true);
          }
          transcode_state.value[hash] = new_value;
        },
        is_finished: (): boolean => {
          const old_value = transcode_state.value[hash];
          if (old_value === undefined) return false;
          return !is_worker_running(old_value.worker_status);
        },
      });
      transcode_background_worker.value[hash] = worker;
    }
    worker.start(restart_if_idle);
  }

  function start_download_background_worker(key: DownloadKey, restart_if_idle?: boolean) {
    let worker = download_background_worker.value[key];
    if (worker === undefined) {
      worker = new BackgroundWorker<DownloadKey, DownloadState>({
        key,
        get_value: (key: DownloadKey) => api.get_download_state(key),
        set_value: (new_value: DownloadState) => {
          const old_value = download_state.value[key];
          if (old_value?.worker_status !== new_value.worker_status) {
            void get_download(key, true);
          }
          download_state.value[key] = new_value;
        },
        is_finished: (): boolean => {
          const old_value = download_state.value[key];
          if (old_value === undefined) return false;
          return !is_worker_running(old_value.worker_status);
        },
      });
      download_background_worker.value[key] = worker;
    }
    worker.start(restart_if_idle);
  }

  async function request_transcode(key: TranscodeKey) {
    const response = await api.request_transcode(key);
    void get_transcode(key, true);
    void get_download(key.video_id, true);
    start_transcode_background_worker(key, true);
    start_download_background_worker(key.video_id, true);
    return response;
  }

  async function get_ytdlp_version(force?: boolean) {
    if (force !== true && ytdlp_version_promise.value !== null) {
      return await ytdlp_version_promise.value;
    }
    const runner = async () => {
      const version = await api.get_ytdlp_version();
      ytdlp_version.value = version;
      return version;
    }
    ytdlp_version_promise.value = runner();
    return await ytdlp_version_promise.value;
  }

  async function request_ytdlp_update(force?: boolean) {
    if (force !== true && ytdlp_version_promise.value !== null) {
      return await ytdlp_version_promise.value;
    }
    const runner = async () => {
      const stdout = await api.request_ytdlp_update();
      ytdlp_update_log.value = stdout;
      return stdout;
    }
    ytdlp_update_promise.value = runner();
    return await ytdlp_update_promise.value;
  }

  return {
    // states
    transcodes,
    downloads,
    transcode_state,
    download_state,
    transcode_background_worker,
    download_background_worker,
    youtube_videos,
    youtube_playlists,
    youtube_videos_error,
    youtube_playlists_error,
    youtube_videos_promise,
    youtube_playlists_promise,
    ytdlp_version,
    ytdlp_update_log,
    ytdlp_version_promise,
    ytdlp_update_promise,
    // actions
    get_transcodes,
    get_transcode,
    delete_transcode,
    get_downloads,
    get_download,
    delete_download,
    get_youtube_video,
    get_youtube_playlist,
    start_transcode_background_worker,
    start_download_background_worker,
    request_transcode,
    get_ytdlp_version,
    request_ytdlp_update,
  }
});

export type CachedApiStore = ReturnType<typeof use_cached_api_store>;
