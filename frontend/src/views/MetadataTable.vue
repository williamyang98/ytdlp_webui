<script setup lang="ts">
import { computed } from "vue";
import { type Metadata } from "../api/youtube_api_schema.ts";
import { type VideoId } from "../api/ytdlp_api_schema.ts";
import { get_youtube_link } from "../api/api.ts";
import { convert_dhms_to_string } from "../utility/format.ts";

const props = defineProps<{
  metadata: Metadata,
  video_id: VideoId,
}>();

const item = computed(() => {
  const item = props.metadata.items.at(0);
  if (item === undefined) return null;
  return item;
});

const thumbnail_link = computed(() => {
  const VALID_THUMBNAILS = ["medium", "standard", "maxres", "high", "default"];
  if (item.value === null) return null;
  const thumbnails = item.value.snippet.thumbnails;
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
  return get_youtube_link(props.video_id);
});
</script>

<template>
<table v-if="item" class="table table-pin-rows table-compact" :class="$attrs.class">
  <tbody>
    <tr>
      <td class="font-medium text-nowrap">Title</td>
      <td>{{ item.snippet.title }}</td>
    </tr>
    <tr>
      <td class="font-medium text-nowrap">Duration</td>
      <td>{{ convert_dhms_to_string(item.contentDetails.duration) }}</td>
    </tr>
    <tr>
      <td class="font-medium text-nowrap">Channel</td>
      <td>{{ item.snippet.channelTitle }}</td>
    </tr>
    <tr>
      <td class="font-medium text-nowrap">Video</td>
      <td><a v-if="youtube_link" class="link link-primary" :href="youtube_link">{{ youtube_link }}</a></td>
    </tr>
    <tr v-if="thumbnail_link">
      <td class="font-medium text-nowrap">Thumbnail</td>
      <td><img :src="thumbnail_link.url" style="max-height: 200px"></td>
    </tr>
    <tr>
      <td class="font-medium text-nowrap">Description</td>
      <td>{{ item.snippet.description }}</td>
    </tr>
  </tbody>
</table>
</template>
