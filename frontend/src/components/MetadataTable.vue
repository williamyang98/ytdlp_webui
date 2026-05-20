<script setup lang="ts">
import { computed } from "vue";
import { get_youtube_link } from "../api/api.ts";
import { type Metadata } from "../api/youtube_api_schema.ts";
import { convert_dhms_to_string, format_date } from "../utility/format.ts";

const props = defineProps<{
  metadata: Metadata,
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
  if (item.value === null) return null;
  return get_youtube_link(item.value.id);
});
</script>

<template>
<div v-if="item !== null" class="w-full">
  <table class="table table-pin-rows table-compact">
    <colgroup>
      <col class="w-px"/>
    </colgroup>
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
        <td class="font-medium text-nowrap">Uploaded</td>
        <td>{{ format_date(item.snippet.publishedAt) }}</td>
      </tr>
      <tr>
        <td class="font-medium text-nowrap">Channel</td>
        <td>{{ item.snippet.channelTitle }}</td>
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
    <p class="text-sm whitespace-pre-wrap">{{ item.snippet.description }}</p>
  </div>
</div>
</template>
