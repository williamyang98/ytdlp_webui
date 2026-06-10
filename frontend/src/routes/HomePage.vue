<script setup lang="ts">
import YoutubeVideoTable from "../components/YoutubeVideoTable.vue";
import YoutubePlaylistTable from "../components/YoutubePlaylistTable.vue";
import RequestBar from "../components/RequestBar.vue";
import DownloadLink from "../components/DownloadLink.vue";
import { use_shared_app_store } from "../stores/shared_app.ts";

const shared_app = use_shared_app_store();

</script>

<template>
<div class="w-full mt-2">
  <RequestBar/>
</div>
<div v-if="shared_app.pending_request !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <DownloadLink :pending_request="shared_app.pending_request"/>
</div>
<div v-if="shared_app.selected_youtube_video !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <YoutubeVideoTable :video_id="shared_app.selected_youtube_video"/>
</div>
<div v-if="shared_app.selected_youtube_playlist !== null" class="w-full">
  <div class="divider my-0.5"></div>
  <YoutubePlaylistTable :playlist_id="shared_app.selected_youtube_playlist"/>
</div>
</template>
