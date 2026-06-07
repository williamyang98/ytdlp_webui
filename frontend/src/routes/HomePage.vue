<script setup lang="ts">
import YoutubeVideoTable from "../components/YoutubeVideoTable.vue";
import YoutubePlaylistTable from "../components/YoutubePlaylistTable.vue";
import RequestBar from "../components/RequestBar.vue";
import DownloadLink from "../components/DownloadLink.vue";
import { providers } from "../providers/providers.ts";
import { RefreshCwIcon } from "lucide-vue-next";

const app = providers.app;
</script>

<template>
<div class="w-full mt-2">
  <RequestBar/>
</div>
<div v-if="app.pending_request !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <DownloadLink :pending_request="app.pending_request"/>
</div>
<div v-if="app.youtube_video !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <div class="w-full flex justify-between px-1">
    <div class="font-medium">Video Information</div>
    <button class="btn btn-sm px-1" @click="app.get_youtube_video(app.youtube_video.id, true)"><RefreshCwIcon class="size-5"/></button>
  </div>
  <YoutubeVideoTable :video="app.youtube_video"/>
</div>
<div v-if="app.youtube_playlist !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <YoutubePlaylistTable :playlist="app.youtube_playlist"/>
</div>
</template>
