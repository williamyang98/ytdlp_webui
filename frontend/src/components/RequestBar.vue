<script setup lang="ts">
import { type TranscodeKey } from "../api/ytdlp_api_schema.ts";
import { computed } from "vue";
import { Search } from "lucide-vue-next";
import { use_cached_api_store } from "../stores/cached_api.ts";
import { use_shared_app_store } from "../stores/shared_app.ts";

const cached_api = use_cached_api_store();
const shared_app = use_shared_app_store();

const request_available = computed(() => {
  const result = shared_app.youtube_url_parse_result;
  return result.video_id !== undefined;
});

const error_message = computed(() => {
  const result = shared_app.youtube_url_parse_result;
  if (result.video_id !== undefined || result.playlist_id !== undefined) return null;
  if (shared_app.youtube_search_bar.url.length > 0) return "Invalid Youtube URL";
  return "Please provide url";
});

function clear() {
  shared_app.youtube_search_bar.url = "";
}

async function submit() {
  const result = shared_app.youtube_url_parse_result;
  if (result.video_id !== undefined) {
    const key: TranscodeKey = {
      video_id: result.video_id,
      audio_ext: shared_app.youtube_search_bar.audio_ext,
    };
    await cached_api.request_transcode(key);
    void cached_api.get_youtube_video(key.video_id);
    shared_app.pending_request = key;
    shared_app.selected_download_key = key.video_id;
    shared_app.selected_transcode_key = key;
  }
}
</script>

<template>
<div class="flex w-full">
  <button class="btn rounded-none rounded-l" @click="clear()" :disabled="shared_app.youtube_search_bar.url.length === 0">Clear</button>
  <div class="grow">
    <label class="input rounded-none w-full" :class="{ 'input-error': error_message !== null}">
      <Search class="text-base-content/50 size-5"/>
      <input
        class="search w-full"
        v-model="shared_app.youtube_search_bar.url" type="text"
        placeholder="Youtube URL"
        required
      />
    </label>
    <label v-if="error_message" class="text-sm text-error text-bold p-1 text-nowrap">{{ error_message }}</label>
  </div>
  <select class="select flex-none w-20 rounded-none" v-model="shared_app.youtube_search_bar.audio_ext">
    <option value="mp3">mp3</option>
    <option value="m4a">m4a</option>
    <option value="webm">webm</option>
    <option value="aac">aac</option>
  </select>
  <button class="btn rounded-none rounded-r" :disabled="!request_available" @click="submit()">Request</button>
</div>
</template>
