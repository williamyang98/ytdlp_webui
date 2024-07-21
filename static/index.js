import { createApp } from "./vendor/vue.esm-browser.prod.js";
import { Api } from "./api.js";
import { Column, ColumnType, Table } from "./table.js";

const WorkerStatus = Object.freeze({
  None: "none",
  Queued: "queued",
  Running: "running",
  Finished: "finished",
  Failed: "failed",
});

const DOWNLOAD_AUDIO_EXT = "m4a";

let extract_youtube_video_id = (url) => {
  const ID_REGEX = /^.*((youtu.be\/)|(v\/)|(\/u\/\w\/)|(embed\/)|(watch\?))\??v?=?([^#&?]*).*/;
  const ID_LENGTH = 11;
  let match = url.match(ID_REGEX);
  if (!match) { return null; }
  let id = match[7];
  if (id.length !== ID_LENGTH) { return null; }
  return id;
}

export const load_html_fragments = async () => {
  let elems = document.querySelectorAll("template[href]");
  let promises = [];
  for (let elem of elems) {
    let promise = async () => {
      let id = elem.getAttribute("id");
      let href = elem.getAttribute("href");
      let res = await fetch(href);
      let body = await res.text();
      elem.innerHTML = body;
      console.log(`Loaded template fragment: id=${id} href=${href}`);
    }
    promises.push(promise());
  }
  return await Promise.all(promises);
}

let unix_time_to_string = (unix_time) => {
  let time = new Date(unix_time * 1000);
  let seconds = time.getSeconds();
  let minutes = time.getMinutes();
  let hours = time.getHours();
  let day = time.getDate();
  let month = time.getMonth()+1; // zero indexed month
  let year = time.getFullYear();
 
  month = String(month).padStart(2,'0');
  day = String(day).padStart(2,'0');
  hours = String(hours).padStart(2,'0');
  minutes = String(minutes).padStart(2,'0');
  seconds = String(seconds).padStart(2,'0');
  return `${year}/${month}/${day}-${hours}:${minutes}:${seconds}`;
}

export let create_app = () => createApp({
  components: {
    "entry-description": {
      props: { entry: Object },
      template: document.querySelector("template#entry-description"),
    },
    "sortable-table": Table,
  },
  data() {
    return {
      transcode_request: {
        url: null,
        format: "mp3",
        url_error: null,
        response: null,
        download_state: null,
        transcode_state: null,
      },
      download_list: [],
      download_list_index: 0,
      transcode_list: [],
      transcode_list_index: 0,
      download_list_columns: [
        new Column("video_id", true, { "type": "text", ignore_case: false }, "Id", ColumnType.TEXT),
        new Column("audio_ext", true, { "type": "text", ignore_case: false }, "Ext", ColumnType.TEXT),
        new Column("status", true, { "type": "text", ignore_case: false }, "Status", ColumnType.TEXT),
        new Column("unix_time", true, null, "Date", ColumnType.DATE, unix_time_to_string),
        new Column("audio_path", false, null, "Audio", ColumnType.LINK),
        new Column("infojson_path", false, null, "Metadata", ColumnType.LINK),
        new Column("stdout_log_path", false, null, "Stdout log", ColumnType.LINK),
        new Column("stderr_log_path", false, null, "Stderr log", ColumnType.LINK),
        new Column("system_log_path", false, null, "System log", ColumnType.LINK),
      ],
      transcode_list_columns: [
        new Column("video_id", true, { "type": "text", ignore_case: false }, "Id", ColumnType.TEXT),
        new Column("audio_ext", true, { "type": "text", ignore_case: false }, "Ext", ColumnType.TEXT),
        new Column("status", true, { "type": "text", ignore_case: false }, "Status", ColumnType.TEXT),
        new Column("unix_time", true, null, "Date", ColumnType.DATE, unix_time_to_string),
        new Column("audio_path", false, null, "Audio", ColumnType.LINK),
        new Column("stdout_log_path", false, null, "Stdout log", ColumnType.LINK),
        new Column("stderr_log_path", false, null, "Stderr log", ColumnType.LINK),
        new Column("system_log_path", false, null, "System log", ColumnType.LINK),
      ],
    }
  },
  computed: {
    download_entry() {
      return this.download_list[this.download_list_index];
    },
    transcode_entry() {
      return this.transcode_list[this.transcode_list_index];
    },
  },
  methods: {
    async refresh_downloads() {
      let res = await Api.get_downloads();
      this.download_list = res;
      this.download_list.sort((a,b) => { return b.unix_time - a.unix_time; });
      this.download_list_index = 0;
    },
    async refresh_transcodes() {
      let res = await Api.get_transcodes();
      this.transcode_list = res;
      this.transcode_list.sort((a,b) => { return b.unix_time - a.unix_time; });
      this.transcode_list_index = 0;
    },
    async request_transcode() {
      if (this.transcode_request.url === null) { return; }
      let id = extract_youtube_video_id(this.transcode_request.url);
      if (id === null) {
        this.transcode_request.url_error = "Invalid Youtube URL"
        return;
      }
      this.transcode_request.url_error = null;
      let res = await Api.request_transcode(id, this.transcode_request.format);
      this.transcode_request.response = res;
    },
    async get_transcode_progress() {
      if (this.transcode_request.url === null) { return; }
      let id = extract_youtube_video_id(this.transcode_request.url);
      if (id === null) {
        this.transcode_request.url_error = "Invalid Youtube URL"
        return;
      }
      this.transcode_request.url_error = null;
      this.transcode_request.download_state = null;
      this.transcode_request.transcode_state = null;
      let res = await Api.get_download_state(id, DOWNLOAD_AUDIO_EXT);
      this.transcode_request.download_state = res;
      if (this.transcode_request.format !== DOWNLOAD_AUDIO_EXT) {
        let res = await Api.get_transcode_state(id, this.transcode_request.format);
        this.transcode_request.transcode_state = res;
      }
    },
    async on_download_table_action({ action, index }) {
      if (action == "delete") {
        let entry = this.download_list[index];
        if (entry !== undefined) {
          await Api.delete_download(entry.video_id, entry.audio_ext);
        }
      }
    },
    async on_transcode_table_action({ action, index }) {
      if (action == "delete") {
        let entry = this.transcode_list[index];
        if (entry !== undefined) {
          await Api.delete_transcode(entry.video_id, entry.audio_ext);
        }
      }
    },
  },
  mounted() {
    this.refresh_downloads();
    this.refresh_transcodes();
  },
})

