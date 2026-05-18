import * as api from "../api/api.ts";
import {
  type YtdlpRow, type FfmpegRow,
  type VideoId, type DownloadKey, type TranscodeKey,
  type TranscodeState, type DownloadState,
  is_worker_running,
} from "../api/ytdlp_api_schema.ts";
import { type Metadata } from "../api/youtube_api_schema.ts";
import { reactive } from "vue";

async function sleep(milliseconds: number) {
  await new Promise(resolve => setTimeout(resolve, milliseconds));
}

export class DownloadWorker {
  key: DownloadKey;
  state: DownloadState | null;
  error: string | null;
  total_requests: number;
  promise: Promise<void> | null;

  constructor(key: DownloadKey) {
    this.key = key;
    this.state = null;
    this.error = null;
    this.total_requests = 0;
    this.promise = null;
  }

  async listen(force?: boolean) {
    if (force !== true && this.promise !== null) {
      return this.promise;
    }
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
  promise: Promise<void> | null;

  constructor(key: TranscodeKey) {
    this.key = key;
    this.state = null;
    this.error = null;
    this.total_requests = 0;
    this.promise = null;
  }

  async listen(force?: boolean) {
    if (force !== true && this.promise !== null) {
      return this.promise;
    }
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
    }
    this.promise = runner();
    return this.promise;
  }
}

function get_transcode_worker_key(key: TranscodeKey): string {
  return `${key.video_id}_${key.audio_ext}`;
}

export class App {
  downloads: YtdlpRow[];
  transcodes: FfmpegRow[];
  selected_download_key: DownloadKey | null;
  selected_transcode_key: TranscodeKey | null;
  pending_request: TranscodeKey | null;
  metadata: Metadata | null;
  download_workers: Partial<Record<string, DownloadWorker>>;
  transcode_workers: Partial<Record<string, TranscodeWorker>>;

  constructor() {
    this.downloads = [];
    this.transcodes = [];
    this.selected_download_key = null;
    this.selected_transcode_key = null;
    this.pending_request = null;
    this.metadata = null;
    this.transcode_workers = {};
    this.download_workers = {};
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

  async get_metadata(video_id: VideoId) {
    const response = await api.get_metadata(video_id);
    this.metadata = response;
  }

  select_download(key: DownloadKey) {
    this.selected_download_key = key;
    const _ = this.get_metadata(key);
  }

  select_transcode(key: TranscodeKey) {
    this.selected_transcode_key = key;
    const _ = this.get_metadata(key.video_id);
  }

  async delete_download(key: DownloadKey) {
    const res = await api.delete_download(key);
    if (res.type === "success") {
      const index = this.downloads.findIndex(v => v.video_id === key);
      if (index >= 0) {
        this.downloads.splice(index, 1);
      }
    }
    this.download_workers[key] = undefined;
    if (this.selected_download_key === key) {
      this.selected_download_key = null;
    }
  }

  async delete_transcode(key: TranscodeKey) {
    const res = await api.delete_transcode(key);
    if (res.type === "success") {
      const index = this.transcodes.findIndex(v => v.video_id === key.video_id && v.audio_ext === key.audio_ext);
      if (index >= 0) {
        this.transcodes.splice(index, 1);
      }
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

  clear_request() {
    this.pending_request = null;
  }

  async request_transcode(key: TranscodeKey) {
    const response = await api.request_transcode(key);
    this.pending_request = key;
    this.selected_download_key = key.video_id;
    this.selected_transcode_key = key;

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
    void this.get_metadata(key.video_id);
    void run_download();
    void run_transcode();
    return response;
  }

  async on_mount() {
    await Promise.all([
      this.get_downloads(),
      this.get_transcodes(),
    ])
  }
}
