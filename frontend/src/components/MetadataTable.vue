<script setup lang="ts">
import { computed } from "vue";
import { get_youtube_link } from "../api/api.ts";
import { convert_dhms_to_string } from "../utility/format.ts";
import { providers } from "../providers/providers.ts";

const app = providers.app;

const item = computed(() => {
  if (app.metadata === null) return null;
  const item = app.metadata.items.at(0);
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
      <td>
        <div class="w-full max-h-50 overflow-auto">
          {{ item.snippet.description }}
        </div>
      </td>
    </tr>
  </tbody>
</table>
</template>
