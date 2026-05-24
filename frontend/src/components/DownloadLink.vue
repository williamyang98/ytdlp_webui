<script setup lang="ts">
import { ref, computed, watch } from "vue";
import TranscodeProgressBar from "./TranscodeProgressBar.vue";
import DownloadProgressBar from "./DownloadProgressBar.vue";
import { type TranscodeKey } from "../api/ytdlp_api_schema.ts";
import { type VideoItem } from "../api/youtube_api_schema.ts";
import { sanitise_to_filepath } from "../utility/format.ts";
import { providers } from "../providers/providers.ts";
import { get_download_link } from "../api/api.ts";
import { HardDriveDownloadIcon } from "lucide-vue-next";

const props = defineProps<{
  pending_request: TranscodeKey,
}>();

const app = providers.app;
const youtube_video = computed(() => app.youtube_video);

const download_name = ref<string>("");

const download_state = computed(() => {
  const download_worker = app.get_download_worker(props.pending_request.video_id);
  return download_worker.state;
});

const transcode_state = computed(() => {
  const transcode_worker = app.get_transcode_worker(props.pending_request);
  return transcode_worker.state;
});

const is_download_ready = computed(() => transcode_state.value?.worker_status === "finished");

function set_download_filename_from_youtube_video(youtube_video: VideoItem): boolean {
  if (youtube_video.id !== props.pending_request.video_id) return false;
  const filename = `${youtube_video.snippet.title}.${props.pending_request.audio_ext}`;
  const sanitised_filename = sanitise_to_filepath(filename);
  download_name.value = sanitised_filename;
  return true;
}

function set_download_filename_from_transcode_key(key: TranscodeKey) {
  download_name.value = `${key.video_id}.${key.audio_ext}`;
}

function download() {
  if (download_name.value.length === 0) return;
  download_name.value = sanitise_to_filepath(download_name.value);
  const link = get_download_link(props.pending_request.video_id, props.pending_request.audio_ext, download_name.value);
  const elem = document.createElement("a");
  elem.href = link;
  elem.rel = "nofollow";
  elem.click();
}

watch(youtube_video, (youtube_video) => {
  if (youtube_video === null) return;
  if (set_download_filename_from_youtube_video(youtube_video)) return;
  set_download_filename_from_transcode_key(props.pending_request);
}, {
  immediate: true,
});

</script>

<template>
<div class="w-full flex flex-col gap-y-1">
  <div class="flex w-full">
    <div class="grow">
      <label class="input rounded-none rounded-l w-full">
        <HardDriveDownloadIcon class="text-base-content/50 size-5"/>
        <input
          class="search w-full"
          placeholder="Filename"
          v-model="download_name" type="text"
        />
      </label>
    </div>
    <button class="btn rounded-none rounded-r" :disabled="!is_download_ready" @click="download">Download</button>
  </div>
  <DownloadProgressBar :state="download_state"/>
  <TranscodeProgressBar :state="transcode_state"/>
</div>
</template>
