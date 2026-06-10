<script setup lang="ts">
import { computed, watch } from "vue";
import TranscodeProgressBar from "./TranscodeProgressBar.vue";
import DownloadProgressBar from "./DownloadProgressBar.vue";
import { type TranscodeKey } from "../api/ytdlp_api_schema.ts";
import { sanitise_to_filepath } from "../utility/format.ts";
import { create_data_url, create_download_link } from "../api/api.ts";
import { DownloadIcon } from "lucide-vue-next";
import { use_cached_api_store, get_transcode_key_hash } from "../stores/cached_api.ts";
import AudioPlayer from "./AudioPlayer.vue";

const props = defineProps<{
  transcode_key: TranscodeKey,
}>();

const cached_api = use_cached_api_store();

const youtube_video = computed(() => {
  const video_id = props.transcode_key.video_id;
  const youtube_video = cached_api.youtube_videos[video_id];
  return youtube_video;
});

const video_id = computed(() => props.transcode_key.video_id);
watch(video_id, (video_id) => {
  void cached_api.get_youtube_video(video_id);
}, {
  immediate: true,
});

const download_name = computed((): string => {
  if (youtube_video.value !== undefined) {
    const filename = `${youtube_video.value.snippet.title}.${props.transcode_key.audio_ext}`;
    const sanitised_filename = sanitise_to_filepath(filename);
    return sanitised_filename;
  }
  const filename = `${props.transcode_key.video_id}.${props.transcode_key.audio_ext}`;
  return filename;
});

type Status = "downloading" | "transcoding" | "download_ready";

const status = computed((): Status => {
  const video_id = props.transcode_key.video_id;
  const download_state = cached_api.download_state[video_id];
  if (download_state?.worker_status !== "finished") return "downloading";

  const hash = get_transcode_key_hash(props.transcode_key);
  const transcode_state = cached_api.transcode_state[hash];
  if (transcode_state?.worker_status !== "finished") return "transcoding";

  return "download_ready";
});

const audio_path = computed(() => {
  const hash = get_transcode_key_hash(props.transcode_key);
  const transcode = cached_api.transcodes[hash];
  return transcode?.audio_path;
});

function download() {
  if (download_name.value.length === 0) return;
  const link = create_download_link(props.transcode_key.video_id, props.transcode_key.audio_ext, download_name.value);
  const elem = document.createElement("a");
  elem.href = link;
  elem.rel = "nofollow";
  elem.click();
}
</script>

<template>
<div class="min-w-[9rem] w-full flex flex-col gap-y-1">
  <template v-if="status === 'downloading'"><DownloadProgressBar :download_key="props.transcode_key.video_id" :hide_subtitle="true"/></template>
  <template v-else-if="status === 'transcoding'"><TranscodeProgressBar :transcode_key="props.transcode_key" :hide_subtitle="true"/></template>
  <div v-else-if="status === 'download_ready'" class="flex w-full">
    <AudioPlayer v-if="audio_path !== undefined" :url="create_data_url(audio_path)" :hide_link="true" :rounded_left="true"/>
    <button class="btn btn-sm px-1 rounded-none rounded-r border-l-0" @click.stop="download"><DownloadIcon class="size-5"/></button>
  </div>
</div>
</template>
