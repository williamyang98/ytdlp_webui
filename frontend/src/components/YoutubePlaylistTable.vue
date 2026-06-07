<script setup lang="ts">
import { type PlaylistItem, type VideoId, type VideoItem } from "../api/youtube_api_schema.ts";
import { type TranscodeKey } from "../api/ytdlp_api_schema.ts";
import * as api from "../api/api.ts";
import { computed, reactive, ref, watch } from "vue";
import { providers } from "../providers/providers.ts";
import { type Playlist } from "../providers/app.ts";
import { convert_dhms_to_string, format_date, type DHMS } from "../utility/format.ts";
import { create_youtube_playlist_link } from "../utility/youtube_url.ts";
import { DownloadIcon, RefreshCwIcon, SettingsIcon } from "lucide-vue-next";
import SortIcon from "./SortIcon.vue";

const app = providers.app;
const props = defineProps<{
  playlist: Playlist,
}>();

// fetch row
const rows = ref<Row[]>([]);
const is_hide_duplicate_videos = ref(true);

class Row {
  index: number;
  video_id: VideoId;
  playlist_item: PlaylistItem;
  video_item: VideoItem | null;
  is_fetch_error: boolean;
  promise: Promise<void> | null;

  constructor(index: number, item: PlaylistItem) {
    this.index = index;
    this.video_id = item.contentDetails.videoId;
    this.playlist_item = item;
    this.video_item = null;
    this.is_fetch_error = false;
    this.promise = null;
  }

  fetch() {
    this.promise = this._fetch();
  }

  async _fetch() {
    try {
      this.video_item = await api.get_youtube_video(this.video_id);
      this.is_fetch_error = false;
    } catch (error) {
      console.error(error);
      this.video_item = null;
      this.is_fetch_error = true;
    }
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
  let items = [];

  if (is_hide_duplicate_videos.value) {
    const video_id_set = new Set<VideoId>();
    for (const item of rows.value) {
      if (video_id_set.has(item.video_id)) continue;
      video_id_set.add(item.video_id);
      items.push(item);
    }
  } else {
    items = [...rows.value];
  }

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

async function download_all() {
  const audio_ext = app.youtube_search_bar.audio_ext;
  const promises = [];
  for (const item of sorted_rows.value) {
    const video_id = item.video_id;
    const key: TranscodeKey = { video_id, audio_ext };
    promises.push(app.request_transcode(key));
  }
  await Promise.all(promises);
}

</script>

<template>
<div class="w-full flex justify-between px-1">
  <div class="font-medium">Video Playlist ({{ sorted_rows.length }})</div>
  <div class="flex px-0">
    <div class="tooltip" data-tip="Download All">
      <button class="btn btn-sm rounded-none rounded-l px-1" @click="download_all"><DownloadIcon class="size-5"/></button>
    </div>
    <div class="tooltip" data-tip="Refresh Playlist">
      <button class="btn btn-sm rounded-none px-1" @click="app.get_youtube_playlist(playlist.id, true)"><RefreshCwIcon class="size-5"/></button>
    </div>
    <button class="btn btn-sm rounded-none rounded-r px-1" popovertarget="popover-1" style="anchor-name:--playlist-settings"><SettingsIcon class="size-5"/></button>
    <ul class="dropdown menu w-52 rounded-box bg-base-100 shadow-sm" popover id="popover-1" style="position-anchor:--playlist-settings">
      <li><label><input type="checkbox" v-model="is_hide_duplicate_videos" class="checkbox"/>Hide duplicate videos</label></li>
    </ul>
  </div>
</div>
<div class="max-h-75 w-full overflow-x-auto">
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
          <template v-if="row.video_item !== null">
            <td>{{ row.video_item.snippet.title }}</td>
            <td>{{ convert_dhms_to_string(row.video_item.contentDetails.duration) }}</td>
            <td><span class="text-nowrap">{{ row.video_item.snippet.channelTitle }}</span></td>
            <td>{{ format_date(row.video_item.snippet.publishedAt) }}</td>
            <td><a class="link link-primary" :href="create_youtube_playlist_link(playlist.id, row.video_id)">Link</a></td>
          </template>
          <template v-else-if="row.is_fetch_error">
            <td colspan="5"><span class="text-nowrap text-error font-medium">Error fetching video information</span></td>
          </template>
          <template v-else>
            <td colspan="5"><span class="text-nowrap font-light">Loading ...</span></td>
          </template>
        </tr>
      </template>
      <template v-if="sorted_rows.length === 0">
        <td colspan="7" class="text-center"><span class="text-nowrap">No videos in playlist</span></td>
      </template>
    </tbody>
  </table>
</div>
</template>
