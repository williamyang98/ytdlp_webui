<script setup lang="ts">
import { computed, onMounted } from "vue";
import { create_youtube_link } from "../utility/youtube_url.ts";
import { type VideoId } from "../api/youtube_api_schema.ts";
import { convert_dhms_to_string, format_date } from "../utility/format.ts";
import { use_cached_api_store } from "../stores/cached_api.ts";
import { RefreshCwIcon } from "lucide-vue-next";

const props = defineProps<{
  video_id: VideoId,
}>();

const cached_api = use_cached_api_store();

const video = computed(() => {
  const videos = cached_api.youtube_videos;
  return videos[props.video_id];
});

onMounted(() => {
  void cached_api.get_youtube_video(props.video_id, false);
});

const thumbnail_link = computed(() => {
  if (video.value === undefined) return null;
  const VALID_THUMBNAILS = ["medium", "standard", "maxres", "high", "default"];
  const thumbnails = video.value.snippet.thumbnails;
  for (const name of VALID_THUMBNAILS) {
    const thumbnail = thumbnails[name];
    if (thumbnail !== undefined) {
      return {
        url: thumbnail.url,
        width: thumbnail.width,
        height: thumbnail.height,
      }
    }
  }
  return null;
});

const youtube_link = computed(() => {
  return create_youtube_link(props.video_id);
});
</script>

<template>
<div class="w-full flex justify-between px-1">
  <div class="font-medium">Video Information</div>
  <button class="btn btn-sm px-1" @click="cached_api.get_youtube_video(video_id, true)"><RefreshCwIcon class="size-5"/></button>
</div>
<template v-if="video !== undefined">
  <table class="table table-pin-rows table-extra-compact">
    <colgroup>
      <col class="w-px"/>
      <col/>
    </colgroup>
    <tbody>
      <tr>
        <td class="font-medium text-nowrap">Title</td>
        <td>{{ video.snippet.title }}</td>
      </tr>
      <tr>
        <td class="font-medium text-nowrap">Duration</td>
        <td>{{ convert_dhms_to_string(video.contentDetails.duration) }}</td>
      </tr>
      <tr>
        <td class="font-medium text-nowrap">Uploaded</td>
        <td>{{ format_date(video.snippet.publishedAt) }}</td>
      </tr>
      <tr>
        <td class="font-medium text-nowrap">Channel</td>
        <td>{{ video.snippet.channelTitle }}</td>
      </tr>
      <tr>
        <td class="font-medium text-nowrap">Video</td>
        <td><a v-if="youtube_link" class="link link-primary" :href="youtube_link">{{ youtube_link }}</a></td>
      </tr>
    </tbody>
  </table>
  <div v-if="thumbnail_link !== null" class="max-w-full">
    <img :src="thumbnail_link.url" class="min-w-xs max-w-lg"/>
  </div>
  <details class="collapse collapse-arrow border border-slate-300 p-2 mt-1" name="youtube-video-description" open>
    <summary class="collapse-title font-semibold p-0 text-sm">Description</summary>
    <div class="collapse-content p-0">
      <div class="w-full max-h-50 overflow-y-auto overflow-x-hidden text-sm whitespace-pre-wrap">
        {{ video.snippet.description }}
      </div>
    </div>
  </details>
</template>
</template>
