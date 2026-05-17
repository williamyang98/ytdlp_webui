<script setup lang="ts">
import UserDataProvider from "./providers/UserDataProvider.vue";
import { MenuIcon } from 'lucide-vue-next';
import GithubIcon from "./assets/github.svg";
import DarkModeToggle from "./utility/DarkModeToggle.vue";
import MetadataTable from "./views/MetadataTable.vue";
import { ref, onMounted } from "vue";

import * as api from "./api/api.ts";
import { type YtdlpRow, type FfmpegRow, type TranscodeKey, type VideoId } from "./api/ytdlp_api_schema.ts";
import { type Metadata } from "./api/youtube_api_schema.ts";
import TranscodeTable from "./views/TranscodeTable.vue";
import DownloadTable from "./views/DownloadTable.vue";

const downloads = ref<YtdlpRow[]>([]);
const transcodes = ref<FfmpegRow[]>([]);
const selected_transcode_key = ref<TranscodeKey | undefined>(undefined);
const selected_download_key = ref<VideoId | undefined>(undefined);

async function get_downloads() {
  const response = await api.get_downloads();
  downloads.value = response;
}

async function get_transcodes() {
  const response = await api.get_transcodes();
  transcodes.value = response;
}

interface SelectedMetadata {
  video_id: VideoId,
  metadata: Metadata,
}
const selected_metadata = ref<SelectedMetadata | null>(null);

async function select_metadata(video_id: VideoId) {
  const response = await api.get_metadata(video_id);
  selected_metadata.value = {
    video_id,
    metadata: response,
  };
}

async function select_download(video_id: VideoId) {
  selected_download_key.value = video_id;
  await select_metadata(video_id);
}

async function select_transcode(key: TranscodeKey) {
  selected_transcode_key.value = key;
  await select_metadata(key.video_id);
}

async function delete_download(video_id: VideoId) {
  const res = await api.delete_download(video_id);
  if (res.type === "success") {
    const index = downloads.value.findIndex(v => v.video_id === video_id);
    if (index >= 0) {
      downloads.value.splice(index, 1);
    }
  }
  if (selected_download_key.value === video_id) {
    selected_download_key.value = undefined;
  }
}

async function delete_transcode(key: TranscodeKey) {
  const res = await api.delete_transcode(key);
  if (res.type === "success") {
    const index = transcodes.value.findIndex(v => v.video_id === key.video_id && v.audio_ext === key.audio_ext);
    if (index >= 0) {
      transcodes.value.splice(index, 1);
    }
  }
  if (selected_transcode_key.value === key) {
    selected_transcode_key.value = undefined;
  }
}

onMounted(async () => {
  await Promise.all([
    get_downloads(),
    get_transcodes(),
  ])
});

</script>

<template>
<UserDataProvider>
<div class="w-screen h-screen overflow-hidden flex flex-col">
  <!-- Navbar -->
  <div class="navbar bg-base-100 shadow-sm min-h-[3rem] p-1">
    <div class="navbar-start w-full md:w-[50%]">
      <!--Mobile hamburger navigation menu-->
      <div class="dropdown lg:hidden">
        <div tabindex="0" role="button" class="btn btn-ghost py-1 px-2">
          <MenuIcon class="w-[1.5rem] h-[1.5rem]"/>
        </div>
        <ul tabindex="0" class="menu dropdown-content bg-base-100 rounded-box z-10 mt-3 min-w-52 p-2 shadow">
        </ul>
      </div>
      <!--Title-->
      <div class="app-title flex flex-row items-center gap-x-2 mx-2">
        <img src="/favicon.png" class="w-[2rem] h-[2rem] flex-none"/>
        <div class="text-lg text-nowrap font-medium">Ytdlp Webui</div>
      </div>
    </div>
    <!--Desktop navigation menu-->
    <div class="navbar-center hidden lg:flex">
      <ul class="menu menu-horizontal px-1 gap-x-1 z-10 p-0">
      </ul>
    </div>
    <div class="navbar-end gap-x-2">
      <a class="cursor-pointer mx-1" href="https://github.com/williamyang98/ytdlp_webui">
        <GithubIcon class="w-[1.75rem] h-[1.75rem]" style="fill: var(--color-base-content)"/>
      </a>
      <DarkModeToggle/>
    </div>
  </div>
  <div class="p-1 flex-1 w-full overflow-auto">
    <div class="inline-flex w-full justify-between">
      <h1 class="text-2xl font-bold">Downloads</h1>
      <button class="btn btn-sm" @click="get_downloads()">Refresh</button>
    </div>
    <DownloadTable :items="downloads" :selected="selected_download_key" @select="select_download" @delete="delete_download"/>
    <br>
    <div class="inline-flex w-full justify-between">
      <h1 class="text-2xl font-bold">Transcodes</h1>
      <button class="btn btn-sm" @click="get_transcodes()">Refresh</button>
    </div>
    <TranscodeTable :items="transcodes" :selected="selected_transcode_key" @select="select_transcode" @delete="delete_transcode"/>
    <template v-if="selected_metadata">
      <MetadataTable :metadata="selected_metadata.metadata" :video_id="selected_metadata.video_id"/>
    </template>
  </div>
</div>
</UserDataProvider>
</template>

<style scoped>
@media (width < 24rem) {
.app-title {
  display: none;
}
}
</style>
