import { createApp } from "./vendor/vue.esm-browser.prod.js";
import { Api } from "./api.js";

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

export let create_app = () => createApp({
  components: {
    "entry-description": {
      props: { entry: Object },
      template: document.querySelector("template#entry-description"),
    },
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
    async delete_transcode() {
      if (this.transcode_request.url === null) { return; }
      let id = extract_youtube_video_id(this.transcode_request.url);
      if (id === null) {
        this.transcode_request.url_error = "Invalid Youtube URL"
        return;
      }
      this.transcode_request.url_error = null;
      let res = await Api.delete_transcode(id, this.transcode_request.format);
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
    }
  },
})

