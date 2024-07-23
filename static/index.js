import { createApp } from "./vendor/vue.esm-browser.prod.js";
import { TranscodeApi, YoutubeApi, WorkerStatus } from "./api.js";
import { Column, ColumnType, Table } from "./fragments/table.js";
import { DownloadProgress } from "./fragments/download_progress.js";
import { TranscodeProgress } from "./fragments/transcode_progress.js";
import { Metadata } from "./fragments/metadata.js";
import { unix_time_to_string, extract_youtube_video_id, sanitise_to_filepath } from "./util.js";

const DOWNLOAD_AUDIO_EXT = "m4a";
const get_cache_key = (video_id, audio_ext) => `${video_id}.${audio_ext}`;

export let create_app = () => createApp({
  components: {
    "sortable-table": Table,
    "download-progress": DownloadProgress,
    "transcode-progress": TranscodeProgress,
    "yt-metadata": Metadata,
  },
  data() {
    return {
      transcode_request: {
        url: null,
        format: "mp3",
        url_error: null,
        pending: false,
      },
      focused_transcode: {
        video_id: null,
        audio_ext: null,
        download_key: null,
        transcode_key: null,
        download_name: null,
      },
      subscribed_transcode_progress_timers: {},
      subscribed_download_progress_timers: {},
      request_id: null,
      metadata: null,
      download_state_cache: {},
      transcode_state_cache: {},
      download_focus_key: null,
      transcode_focus_key: null,
      download_progress_cache: {},
      transcode_progress_cache: {},
      download_state_columns: [
        new Column("video_id", true, { "type": "text", ignore_case: false }, "Id", ColumnType.TEXT),
        new Column("audio_ext", true, { "type": "text", ignore_case: false }, "Ext", ColumnType.TEXT),
        new Column("status", true, { "type": "text", ignore_case: false }, "Status", ColumnType.TEXT),
        new Column("unix_time", true, null, "Date", ColumnType.DATE, unix_time_to_string),
        new Column("audio_path", false, null, "Audio", ColumnType.LINK),
        new Column("infojson_path", false, null, "Info", ColumnType.LINK),
        new Column("stdout_log_path", false, null, "Stdout", ColumnType.LINK),
        new Column("stderr_log_path", false, null, "Stderr", ColumnType.LINK),
        new Column("system_log_path", false, null, "System", ColumnType.LINK),
      ],
      transcode_state_columns: [
        new Column("video_id", true, { "type": "text", ignore_case: false }, "Id", ColumnType.TEXT),
        new Column("audio_ext", true, { "type": "text", ignore_case: false }, "Ext", ColumnType.TEXT),
        new Column("status", true, { "type": "text", ignore_case: false }, "Status", ColumnType.TEXT),
        new Column("unix_time", true, null, "Date", ColumnType.DATE, unix_time_to_string),
        new Column("audio_path", false, null, "Audio", ColumnType.LINK),
        new Column("stdout_log_path", false, null, "Stdout", ColumnType.LINK),
        new Column("stderr_log_path", false, null, "Stderr", ColumnType.LINK),
        new Column("system_log_path", false, null, "System", ColumnType.LINK),
      ],
    }
  },
  computed: {
    disable_submit() {
      if (this.request_id === null) return true;
      if (this.transcode_request.pending) return true;
      if (this.metadata === null) return true;
      return false;
    },
    download_link() {
      if (this.disable_submit) return null;
      let video_id = this.focused_transcode.video_id;
      let audio_ext = this.focused_transcode.audio_ext;
      let key = get_cache_key(video_id, audio_ext);
      if (video_id === null) return null;
      let cache = (audio_ext == DOWNLOAD_AUDIO_EXT) ? this.download_state_cache : this.transcode_state_cache;
      let is_ready = cache[key]?.status == WorkerStatus.Finished;
      if (!is_ready) return null;
      return TranscodeApi.get_download_link(video_id, audio_ext, this.focused_transcode.download_name);
    },
  },
  methods: {
    async update_request_id() {
      if (this.transcode_request.url === null) {
        this.request_id = null;
        this.transcode_request.url_error = "Please provide URL"
        return;
      }
      this.request_id = extract_youtube_video_id(this.transcode_request.url);
      if (this.request_id === null) {
        this.transcode_request.url_error = "Invalid Youtube URL"
        return;
      }
      // avoid loading metadata if already loaded
      if (this.metadata?.id == this.request_id) {
        this.transcode_request.url_error = null;
        return;
      }
      await this.load_metadata();
    },
    async load_metadata() {
      this.metadata = null;
      let res = await YoutubeApi.get_metadata(this.request_id);
      if (res === null) {
        this.transcode_request.url_error = "Youtube metadata was not found";
        return;
      }
      this.transcode_request.url_error = null;
      this.metadata = res;
    },
    async try_request_transcode() {
      if (this.transcode_request.pending) return;
      this.transcode_request.pending = true;
      await this.request_transcode();
      this.transcode_request.pending = false;
    },
    async request_transcode() {
      if (this.request_id === null) return;
      let video_id = this.request_id;
      let audio_ext = this.transcode_request.format;
      this.transcode_request.url_error = null;
      await TranscodeApi.request_transcode(video_id, audio_ext);
      this.update_focused_transcode(video_id, audio_ext);
    },
    async update_focused_transcode(video_id, audio_ext) {
      let download_name = `${sanitise_to_filepath(this.metadata.snippet.title)}.${audio_ext}`;
      this.focused_transcode = { 
        video_id, audio_ext, download_name,
        download_key: get_cache_key(video_id, DOWNLOAD_AUDIO_EXT),
        transcode_key: (audio_ext == DOWNLOAD_AUDIO_EXT) ? null : get_cache_key(video_id, audio_ext),
      };
      this.subscribe_to_download(video_id, DOWNLOAD_AUDIO_EXT);
      if (audio_ext != DOWNLOAD_AUDIO_EXT) {
        this.subscribe_to_transcode(video_id, audio_ext);
      }
      // select entry from list
      this.download_focus_key = get_cache_key(video_id, DOWNLOAD_AUDIO_EXT);
      if (audio_ext != DOWNLOAD_AUDIO_EXT) {
        this.transcode_focus_key = get_cache_key(video_id, audio_ext);
      }
    },
    async refresh_downloads() {
      let res = await TranscodeApi.get_downloads();
      for (let state of res) {
        let key = get_cache_key(state.video_id, state.audio_ext);
        this.download_state_cache[key] = state;
      }
    },
    async refresh_transcodes() {
      let res = await TranscodeApi.get_transcodes();
      for (let state of res) {
        let key = get_cache_key(state.video_id, state.audio_ext);
        this.transcode_state_cache[key] = state;
      }
    },
    async on_download_select(entry) {
      this.download_focus_key = get_cache_key(entry.video_id, entry.audio_ext);
      this.subscribe_to_download(entry.video_id, entry.audio_ext);
    },
    async on_transcode_select(entry) {
      this.transcode_focus_key = get_cache_key(entry.video_id, entry.audio_ext);
      this.subscribe_to_transcode(entry.video_id, entry.audio_ext);
    },
    async on_download_table_action({ action, entry }) {
      if (action == "delete") {
        let res = await TranscodeApi.delete_download(entry.video_id, entry.audio_ext);
        if (res.type == "success") {
          let key = get_cache_key(entry.video_id, entry.audio_ext);
          if (this.download_focus_key == key) this.download_focus_key = null;
          delete this.download_state_cache[key];
          delete this.download_progress_cache[key];
        }
      }
    },
    async on_transcode_table_action({ action, entry }) {
      if (action == "delete") {
        let res = await TranscodeApi.delete_transcode(entry.video_id, entry.audio_ext);
        if (res.type == "success") {
          let key = get_cache_key(entry.video_id, entry.audio_ext);
          if (this.transcode_focus_key == key) this.transcode_focus_key = null;
          delete this.transcode_state_cache[key];
          delete this.transcode_progress_cache[key];
        }
      }
    },
    subscribe_to_download(video_id, audio_ext) {
      let key = get_cache_key(video_id, audio_ext);
      let timers = this.subscribed_download_progress_timers;
      if (timers[key] !== undefined) return false;
      const is_worker_finished = (status) => {
        return (status == WorkerStatus.Failed) || (status == WorkerStatus.Finished);
      };
      let timer_handle = setInterval(async () => {
        let is_finished = false;
        try {
          let progress = await TranscodeApi.get_download_progress(video_id, audio_ext);
          this.download_progress_cache[key] = progress;
          let state = this.download_state_cache[key];
          if (state?.status != progress.worker_status) {
            let state = await TranscodeApi.get_download(video_id, audio_ext);
            this.download_state_cache[key] = state;
          }
          is_finished = is_worker_finished(progress.worker_status);
        } catch {
          is_finished = true;
        }
        if (is_finished) {
          clearInterval(timer_handle);
          timers[key] = undefined;
        }
      }, 1000);
      timers[key] = timer_handle;
      return true;
    },
    subscribe_to_transcode(video_id, audio_ext) {
      let key = get_cache_key(video_id, audio_ext);
      let timers = this.subscribed_transcode_progress_timers;
      if (timers[key] !== undefined) return false;
      const is_worker_finished = (status) => {
        return (status == WorkerStatus.Failed) || (status == WorkerStatus.Finished);
      };
      let timer_handle = setInterval(async () => {
        let is_finished = false;
        try {
          let progress = await TranscodeApi.get_transcode_progress(video_id, audio_ext);
          this.transcode_progress_cache[key] = progress;
          let state = this.transcode_state_cache[key];
          if (state?.status != progress.worker_status) {
            let state = await TranscodeApi.get_transcode(video_id, audio_ext);
            this.transcode_state_cache[key] = state;
          }
          is_finished = is_worker_finished(progress.worker_status);
        } catch {
          is_finished = true;
        }
        if (is_finished) {
          clearInterval(timer_handle);
          timers[key] = undefined;
        }
      }, 1000);
      timers[key] = timer_handle;
      return true;
    },
    async download_file() {
      if (this.download_link === null) return;
      let elem = document.createElement("a");
      elem.href = this.download_link;
      elem.rel = "nofollow";
      elem.click();
    },
  },
  mounted() {
    this.update_request_id();
    this.refresh_downloads();
    this.refresh_transcodes();
  },
})

