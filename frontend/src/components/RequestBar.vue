<script setup lang="ts">
import { type AudioExtension, type TranscodeKey } from "../api/ytdlp_api_schema.ts";
import { providers } from "../providers/providers.ts";
import { extract_youtube_video_id } from "../utility/format.ts";
import { ref, computed, watch } from "vue";
import { Search } from "lucide-vue-next";

const app = providers.app;

const search_value = ref<string>("");
const audio_ext = ref<AudioExtension>("mp3");
const video_id = computed(() => extract_youtube_video_id(search_value.value));
const request_available = computed(() => video_id.value !== null);

const error_message = computed(() => {
  if (video_id.value !== null) return null;
  if (search_value.value.length > 0) return "Invalid Youtube URL";
  return "Please provide url";
});

function clear() {
  search_value.value = "";
  app.clear_request();
}

async function submit() {
  if (video_id.value === null) return;
  const key: TranscodeKey = {
    video_id: video_id.value,
    audio_ext: audio_ext.value,
  };
  await app.request_transcode(key);
}

watch(search_value, (search_value) => {
  if (search_value.length === 0) {
    app.clear_request();
  }
})

watch(video_id, (video_id) => {
  if (video_id === null) return;
  const _ = app.get_metadata(video_id);
});

</script>

<template>
<div class="flex w-full">
  <button class="btn rounded-none rounded-l" @click="clear()" :disabled="search_value.length === 0">Clear</button>
  <div class="grow">
    <label class="input rounded-none w-full" :class="{ 'input-error': error_message !== null}">
      <Search class="text-base-content/50 size-5"/>
      <input
        class="search w-full"
        v-model="search_value" type="text"
        placeholder="Youtube URL"
        required
      />
    </label>
    <label v-if="error_message" class="text-sm text-error text-bold p-1 text-nowrap">{{ error_message }}</label>
  </div>
  <select class="select flex-none w-23 rounded-none" v-model="audio_ext">
    <option value="mp3">mp3</option>
    <option value="m4a">m4a</option>
    <option value="webm">webm</option>
    <option value="aac">aac</option>
  </select>
  <button class="btn rounded-none rounded-r" :disabled="!request_available" @click="submit()">Request</button>
</div>
</template>
