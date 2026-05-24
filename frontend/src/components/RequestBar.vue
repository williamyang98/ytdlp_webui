<script setup lang="ts">
import { type TranscodeKey } from "../api/ytdlp_api_schema.ts";
import { providers } from "../providers/providers.ts";
import { computed } from "vue";
import { Search } from "lucide-vue-next";

const app = providers.app;
const request_available = computed(() => {
  const result = app.youtube_url_parse_result;
  return result.video_id !== undefined;
});

const error_message = computed(() => {
  const result = app.youtube_url_parse_result;
  if (result.video_id !== undefined || result.playlist_id !== undefined) return null;
  if (app.youtube_search_bar.url.length > 0) return "Invalid Youtube URL";
  return "Please provide url";
});

function clear() {
  app.youtube_search_bar.url = "";
}

async function submit() {
  const result = app.youtube_url_parse_result;
  if (result.video_id !== undefined) {
    const key: TranscodeKey = {
      video_id: result.video_id,
      audio_ext: app.youtube_search_bar.audio_ext,
    };
    await app.request_transcode(key);
  }
}
</script>

<template>
<div class="flex w-full">
  <button class="btn rounded-none rounded-l" @click="clear()" :disabled="app.youtube_search_bar.url.length === 0">Clear</button>
  <div class="grow">
    <label class="input rounded-none w-full" :class="{ 'input-error': error_message !== null}">
      <Search class="text-base-content/50 size-5"/>
      <input
        class="search w-full"
        v-model="app.youtube_search_bar.url" type="text"
        placeholder="Youtube URL"
        required
      />
    </label>
    <label v-if="error_message" class="text-sm text-error text-bold p-1 text-nowrap">{{ error_message }}</label>
  </div>
  <select class="select flex-none w-20 rounded-none" v-model="app.youtube_search_bar.audio_ext">
    <option value="mp3">mp3</option>
    <option value="m4a">m4a</option>
    <option value="webm">webm</option>
    <option value="aac">aac</option>
  </select>
  <button class="btn rounded-none rounded-r" :disabled="!request_available" @click="submit()">Request</button>
</div>
</template>
