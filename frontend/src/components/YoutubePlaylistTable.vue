<script setup lang="ts">
import { type PlaylistItem, type VideoId, type VideoItem } from "../api/youtube_api_schema.ts";
import * as api from "../api/api.ts";
import { computed, reactive, ref, watch } from "vue";
import { providers } from "../providers/providers.ts";
import { type Playlist } from "../providers/app.ts";
import { convert_dhms_to_string, format_date, type DHMS } from "../utility/format.ts";
import { create_youtube_playlist_link } from "../utility/youtube_url.ts";
import SortIcon from "./SortIcon.vue";

const app = providers.app;
const props = defineProps<{
  playlist: Playlist,
}>();

// fetch row
const rows = ref<Row[]>([]);

class Row {
  index: number;
  video_id: VideoId;
  playlist_item: PlaylistItem;
  video_item: VideoItem | null;
  promise: Promise<void> | null;

  constructor(index: number, item: PlaylistItem) {
    this.index = index;
    this.video_id = item.contentDetails.videoId;
    this.playlist_item = item;
    this.video_item = null;
    this.promise = null;
  }

  fetch() {
    this.promise = this._fetch();
  }

  async _fetch() {
    const video_item = await api.get_youtube_video(this.video_id);
    this.video_item = video_item;
  }
}

watch(props.playlist, (playlist) => {
  rows.value = playlist.items.map((item, index) => {
    const row = reactive(new Row(index, item));
    row.fetch();
    return row;
  });
}, {
  immediate: true,
});

// sort rows
type Column = "index" | "video_id" | "title" | "duration" | "channel" | "published_at";

interface Order {
  column: Column,
  is_descending: boolean,
}

const sort_order = ref<Order>({
  column: "index",
  is_descending: false,
});

function dhms_to_seconds(dhms: DHMS): number {
  return dhms.seconds + dhms.minutes*60 + dhms.hours*60*60 + dhms.days*60*60*24;
}

const sorted_rows = computed(() => {
  const column = sort_order.value.column;
  const is_descending = sort_order.value.is_descending;
  const items = [...rows.value];
  switch (column) {
    case "index": {
      items.sort((a, b) => a.index - b.index);
      break;
    }
    case "video_id": {
      items.sort((a, b) => a.video_id.localeCompare(b.video_id));
      break;
    }
    case "title": {
      items.sort((a, b) => {
        if (a.video_item === null || b.video_item === null) return 0;
        return a.video_item.snippet.title.localeCompare(b.video_item.snippet.title);
      });
      break;
    }
    case "duration": {
      items.sort((a, b) => {
        if (a.video_item === null || b.video_item === null) return 0;
        return dhms_to_seconds(a.video_item.contentDetails.duration)-dhms_to_seconds(b.video_item.contentDetails.duration);
      });
      break;
    }
    case "channel": {
      items.sort((a, b) => {
        if (a.video_item === null || b.video_item === null) return 0;
        return a.video_item.snippet.channelTitle.localeCompare(b.video_item.snippet.channelTitle);
      });
      break;
    }
    case "published_at": {
      items.sort((a, b) => {
        if (a.video_item === null || b.video_item === null) return 0;
        return a.video_item.snippet.publishedAt.getTime()-b.video_item.snippet.publishedAt.getTime();
      });
      break;
    }
  }
  if (is_descending) {
    items.reverse();
  }
  return items;
});


function click_sort_column(column: Column) {
  // toggle
  if (sort_order.value.column === column) {
    sort_order.value.is_descending = !sort_order.value.is_descending;
    return;
  }
  // default
  sort_order.value = {
    column,
    is_descending: true,
  };
}

function get_sort_icon_mode(column: Column): boolean | undefined {
  if (sort_order.value.column !== column) return undefined;
  return sort_order.value.is_descending;
}

// select playlist item
function get_playlist_item_class(row: Row): string {
  const youtube_video = app.youtube_video;
  if (youtube_video === null) return "";
  const is_selected = row.video_id === youtube_video.id;
  return is_selected ? "bg-base-300" : "";
}

function select_playlist_item(row: Row) {
  app.select_playlist_item(props.playlist.id, row.video_id);
}

</script>

<template>
<table class="table table-pin-rows table-pin-cols table-extra-compact">
  <colgroup>
    <col class="w-px"/>
    <col class="w-px"/>
    <col class="min-w-50 w-full"/>
    <col class="w-px"/>
    <col class="w-px"/>
    <col class="w-px"/>
  </colgroup>
  <thead>
    <tr>
      <th>
        <div class="inline-flex gap-2">
          <div @click="click_sort_column('index')"><SortIcon :is_descending="get_sort_icon_mode('index')"/></div>
        </div>
      </th>
      <th>
        <div class="inline-flex gap-2">
          <div>Video ID</div>
          <div @click="click_sort_column('video_id')"><SortIcon :is_descending="get_sort_icon_mode('video_id')"/></div>
        </div>
      </th>
      <th>
        <div class="inline-flex gap-2">
          <div>Title</div>
          <div @click="click_sort_column('title')"><SortIcon :is_descending="get_sort_icon_mode('title')"/></div>
        </div>
      </th>
      <th>
        <div class="inline-flex gap-2">
          <div>Duration</div>
          <div @click="click_sort_column('duration')"><SortIcon :is_descending="get_sort_icon_mode('duration')"/></div>
        </div>
      </th>
      <th>
        <div class="inline-flex gap-2">
          <div>Channel</div>
          <div @click="click_sort_column('channel')"><SortIcon :is_descending="get_sort_icon_mode('channel')"/></div>
        </div>
      </th>
      <th>
        <div class="inline-flex gap-2">
          <div>Published At</div>
          <div @click="click_sort_column('published_at')"><SortIcon :is_descending="get_sort_icon_mode('published_at')"/></div>
        </div>
      </th>
      <th>Link</th>
    </tr>
  </thead>
  <tbody>
    <template v-for="row in sorted_rows" :key="row.index">
      <tr
        class="hover:bg-base-300 cursor-pointer"
        :class="get_playlist_item_class(row)"
        @click="select_playlist_item(row)"
      >
        <th>{{ row.index+1 }}</th>
        <td>{{ row.video_id }}</td>
        <td>
          <template v-if="row.video_item !== null">{{ row.video_item.snippet.title }}</template>
          <template v-else>...</template>
        </td>
        <td>
          <template v-if="row.video_item !== null">{{ convert_dhms_to_string(row.video_item.contentDetails.duration) }}</template>
          <template v-else>...</template>
        </td>
        <td>
          <span v-if="row.video_item !== null" class="text-nowrap">{{ row.video_item.snippet.channelTitle }}</span>
          <template v-else>...</template>
        </td>
        <td>
          <template v-if="row.video_item !== null">{{ format_date(row.video_item.snippet.publishedAt) }}</template>
          <template v-else>...</template>
        </td>
        <td>
          <a v-if="row.video_item !== null" class="link link-primary" :href="create_youtube_playlist_link(playlist.id, row.video_id)">Link</a>
          <template v-else>...</template>
        </td>
      </tr>
    </template>
  </tbody>
</table>
</template>
