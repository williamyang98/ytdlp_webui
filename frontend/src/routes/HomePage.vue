<script setup lang="ts">
import YoutubeVideoTable from "../components/YoutubeVideoTable.vue";
import YoutubePlaylistTable from "../components/YoutubePlaylistTable.vue";
import RequestBar from "../components/RequestBar.vue";
import DownloadLink from "../components/DownloadLink.vue";
import { providers } from "../providers/providers.ts";

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
<div v-if="app.selected_youtube_video !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <YoutubeVideoTable :video_id="app.selected_youtube_video"/>
</div>
<div v-if="app.selected_youtube_playlist !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <YoutubePlaylistTable :playlist_id="app.selected_youtube_playlist"/>
</div>
</template>
