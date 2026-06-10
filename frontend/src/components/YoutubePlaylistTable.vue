<script setup lang="ts">
import { type PlaylistId, type PlaylistItem, type VideoId, type VideoItem } from "../api/youtube_api_schema.ts";
import { type TranscodeKey, type WorkerStatus } from "../api/ytdlp_api_schema.ts";
import { type ComputedRef, computed, ref } from "vue";
import { convert_dhms_to_string, format_date, type DHMS } from "../utility/format.ts";
import { create_youtube_playlist_link } from "../utility/youtube_url.ts";
import { DownloadIcon, RefreshCwIcon, SettingsIcon } from "lucide-vue-next";
import SortIcon from "./SortIcon.vue";
import { get_transcode_key_hash, use_cached_api_store } from "../stores/cached_api.ts";
import { use_shared_app_store } from "../stores/shared_app.ts";
import InlineDownloadLink from "./InlineDownloadLink.vue";

const cached_api = use_cached_api_store();
const shared_app = use_shared_app_store();

const props = defineProps<{
  playlist_id: PlaylistId,
}>();

const is_hide_duplicate_videos = ref(true);

// granular ordering of transcode requests
type Phase =
  { type: "downloading", status?: WorkerStatus } |
  { type: "transcoding", status?: WorkerStatus } |
  { type: "download_ready" };

function get_worker_status_value(status?: WorkerStatus): number {
  switch (status) {
    case undefined: return 0;
    case "queued": return 1;
    case "running": return 2;
    case "failed": return 3;
    case "finished": return 4;
  }
}

function get_phase_value(phase: Phase): number {
  switch (phase.type) {
    case "downloading": return get_worker_status_value(phase.status);
    case "transcoding": return get_worker_status_value(phase.status) + 10;
    case "download_ready": return 20;
  }
}

function get_phase(video_id: VideoId): Phase {
  const download_state = cached_api.download_state[video_id];
  if (download_state?.worker_status !== "finished") {
    return { type: "downloading", status: download_state?.worker_status };
  }

  const transcode_key: TranscodeKey = { video_id, audio_ext: shared_app.youtube_search_bar.audio_ext };
  const hash = get_transcode_key_hash(transcode_key);
  const transcode_state = cached_api.transcode_state[hash];
  if (transcode_state?.worker_status !== "finished") {
    return { type: "transcoding", status: transcode_state?.worker_status };
  }
  return { type: "download_ready" };
}

class Row {
  index: number;
  video_id: VideoId;
  playlist_item: PlaylistItem;
  video_item: ComputedRef<VideoItem | undefined>;
  phase: ComputedRef<Phase>;
  video_error: ComputedRef<string | undefined>;

  constructor(index: number, item: PlaylistItem) {
    const video_id = item.contentDetails.videoId;
    this.index = index;
    this.video_id = video_id;
    this.playlist_item = item;
    this.video_item = computed(() => cached_api.youtube_videos[this.video_id]);
    this.phase = computed(() => get_phase(this.video_id));
    this.video_error = computed(() => cached_api.youtube_videos_error[this.video_id]);
    void cached_api.get_youtube_video(video_id);
  }
}

const rows = computed(() => {
  const playlist_items = cached_api.youtube_playlists[props.playlist_id];
  if (playlist_items === undefined) return [];
  return playlist_items.map((item, index) => new Row(index, item));
});

// sort rows
type Column = "index" | "video_id" | "title" | "duration" | "channel" | "published_at" | "status";

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

  function sort_out_missing_youtube_metadata(compare_function: (a_metadata: VideoItem, b_metadata: VideoItem, a: Row, b: Row) => number) {
    return (a: Row, b: Row): number => {
      if (a.video_item.value !== undefined && b.video_item.value !== undefined) {
        return compare_function(a.video_item.value, b.video_item.value, a, b);
      }
      if (a.video_item.value !== undefined && b.video_item.value === undefined) return 1;
      if (a.video_item.value === undefined && b.video_item.value !== undefined) return -1;
      return 0;
    }
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
      items.sort(sort_out_missing_youtube_metadata((a, b) => a.snippet.title.localeCompare(b.snippet.title)));
      break;
    }
    case "duration": {
      items.sort(sort_out_missing_youtube_metadata((a, b) => {
        return dhms_to_seconds(a.contentDetails.duration)-dhms_to_seconds(b.contentDetails.duration);
      }));
      break;
    }
    case "channel": {
      items.sort(sort_out_missing_youtube_metadata((a, b) => a.snippet.channelTitle.localeCompare(b.snippet.channelTitle)));
      break;
    }
    case "published_at": {
      items.sort(sort_out_missing_youtube_metadata((a, b) => {
        return a.snippet.publishedAt.getTime()-b.snippet.publishedAt.getTime();
      }));
      break;
    }
    case "status": {
      items.sort(sort_out_missing_youtube_metadata((_a_metadata, _b_metadata, a, b) => {
        return get_phase_value(a.phase.value)-get_phase_value(b.phase.value);
      }));
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
  const video_id = shared_app.selected_youtube_video;
  if (video_id === null) return "";
  const is_selected = row.video_id === video_id;
  return is_selected ? "bg-base-300" : "";
}

function select_youtube_playlist_item(row: Row) {
  shared_app.select_youtube_playlist_item(props.playlist_id, row.video_id);
}

async function download_all() {
  const audio_ext = shared_app.youtube_search_bar.audio_ext;
  const promises = [];
  for (const item of sorted_rows.value) {
    const video_id = item.video_id;
    const key: TranscodeKey = { video_id, audio_ext };
    promises.push(cached_api.request_transcode(key));
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
      <button class="btn btn-sm rounded-none px-1" @click="cached_api.get_youtube_playlist(playlist_id, true)"><RefreshCwIcon class="size-5"/></button>
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
      <col class="w-full"/>
      <col class="w-px"/>
      <col class="w-px"/>
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
            <div>Status</div>
            <div @click="click_sort_column('status')"><SortIcon :is_descending="get_sort_icon_mode('status')"/></div>
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
          @click="select_youtube_playlist_item(row)"
        >
          <th>{{ row.index+1 }}</th>
          <td><span class="text-nowrap">{{ row.video_id }}</span></td>
          <template v-if="row.video_item.value !== undefined">
            <td><div>{{ row.video_item.value.snippet.title }}</div></td>
            <td><InlineDownloadLink :transcode_key="{ video_id: row.video_id, audio_ext: shared_app.youtube_search_bar.audio_ext }"/></td>
            <td>{{ convert_dhms_to_string(row.video_item.value.contentDetails.duration) }}</td>
            <td><span class="text-nowrap">{{ row.video_item.value.snippet.channelTitle }}</span></td>
            <td>{{ format_date(row.video_item.value.snippet.publishedAt) }}</td>
            <td><a class="link link-primary" :href="create_youtube_playlist_link(playlist_id, row.video_id)">Link</a></td>
          </template>
          <template v-else-if="row.video_error.value !== undefined">
            <td colspan="6"><span class="text-nowrap text-error font-medium">Error fetching video information: {{ row.video_error.value }}</span></td>
          </template>
          <template v-else>
            <td colspan="6"><span class="text-nowrap font-light">Loading ...</span></td>
          </template>
        </tr>
      </template>
      <template v-if="sorted_rows.length === 0">
        <td colspan="8" class="text-center"><span class="text-nowrap">No videos in playlist</span></td>
      </template>
    </tbody>
  </table>
</div>
</template>
