<script setup lang="ts">
import { computed } from "vue";
import { get_youtube_link } from "../api/api.ts";
import { type VideoItem } from "../api/youtube_api_schema.ts";
import { convert_dhms_to_string, format_date } from "../utility/format.ts";

const props = defineProps<{
  video: VideoItem,
}>();

const thumbnail_link = computed(() => {
  const VALID_THUMBNAILS = ["medium", "standard", "maxres", "high", "default"];
  const thumbnails = props.video.snippet.thumbnails;
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
  return get_youtube_link(props.video.id);
});
</script>

<template>
<table class="table table-pin-rows table-compact">
  <colgroup>
    <col class="w-px"/>
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
  <img :src="thumbnail_link.url" class="w-full max-w-lg"/>
</div>
<div class="w-full max-h-100 overflow-y-auto overflow-x-hidden">
  <span class="font-medium text-nowrap">Description</span>
  <p class="text-sm whitespace-pre-wrap">{{ video.snippet.description }}</p>
</div>
</template>
