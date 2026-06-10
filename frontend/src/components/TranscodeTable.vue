<script setup lang="ts">
import SortIcon from "./SortIcon.vue";
import TranscodeProgressBar from "./TranscodeProgressBar.vue";
import { FileTerminal, RefreshCwIcon, Trash2 } from "lucide-vue-next";
import AudioPlayer from "./AudioPlayer.vue";
import { type TranscodeKey, type FfmpegRow } from "../api/ytdlp_api_schema.ts";
import { format_datetime } from "../utility/format.ts";
import { create_data_url } from "../api/api.ts";
import { ref, computed, type ComputedRef } from "vue";
import { is_worker_running } from "../api/ytdlp_api_schema.ts";
import { use_cached_api_store } from "../stores/cached_api.ts";
import { use_shared_app_store } from "../stores/shared_app.ts";
import type { VideoItem } from "../api/youtube_api_schema.ts";

const cached_api = use_cached_api_store();
const shared_app = use_shared_app_store();

const sort_order = ref<Order>({
  column: "time",
  is_descending: true,
});

type Column = "video_id" | "audio_ext" | "status" | "time" | "title";
interface Order {
  column: Column,
  is_descending: boolean,
}

function select_transcode(row: FfmpegRow) {
  const key: TranscodeKey = {
    video_id: row.video_id,
    audio_ext: row.audio_ext,
  };
  shared_app.select_transcode(key);
}

async function delete_transcode(row: FfmpegRow) {
  const key: TranscodeKey = {
    video_id: row.video_id,
    audio_ext: row.audio_ext,
  };
  await cached_api.delete_transcode(key);
}

function get_selected_class(row: FfmpegRow): string {
  const key = shared_app.selected_transcode_key;
  if (key === null) return "";
  const is_selected = row.video_id === key.video_id && row.audio_ext === key.audio_ext;
  return is_selected ? "bg-base-300" : "";
}

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

class Row {
  state: FfmpegRow;
  metadata: ComputedRef<VideoItem | undefined>;

  constructor(state: FfmpegRow) {
    const video_id = state.video_id;
    this.state = state;
    void cached_api.get_youtube_video(video_id);
    this.metadata = computed(() => {
      return cached_api.youtube_videos[video_id];
    });
  }
}

const items = computed(() => {
  return Object.values(cached_api.transcodes)
    .filter(v => v !== undefined)
    .map(v => new Row(v));
});

const sorted_items = computed(() => {
  const column = sort_order.value.column;
  const is_descending = sort_order.value.is_descending;
  const sorted_items = [...items.value];
  switch (column) {
    case "video_id": {
      sorted_items.sort((a, b) => a.state.video_id.localeCompare(b.state.video_id));
      break;
    }
    case "audio_ext": {
      sorted_items.sort((a, b) => a.state.audio_ext.localeCompare(b.state.audio_ext));
      break;
    }
    case "status": {
      sorted_items.sort((a, b) => a.state.status.localeCompare(b.state.status));
      break;
    }
    case "time": {
      sorted_items.sort((a, b) => a.state.unix_time.getTime()-b.state.unix_time.getTime());
      break;
    }
    case "title": {
      sorted_items.sort((a, b) => {
        const a_metadata = a.metadata.value;
        const b_metadata = b.metadata.value;
        if (a_metadata !== undefined && b_metadata !== undefined) {
          return a_metadata.snippet.title.localeCompare(b_metadata.snippet.title);
        }
        if (a_metadata !== undefined && b_metadata === undefined) return -1;
        if (a_metadata === undefined && b_metadata !== undefined) return 1;
        return 0;
      });
      break;
    }
  }
  if (is_descending) {
    sorted_items.reverse();
  }
  return sorted_items;
});

</script>

<template>
<div class="inline-flex w-full justify-between py-1">
  <h1 class="text-xl font-bold">Transcodes ({{ sorted_items.length }})</h1>
  <button class="btn btn-sm px-1" @click="cached_api.get_transcodes(true)"><RefreshCwIcon class="size-5"/></button>
</div>
<TranscodeProgressBar v-if="shared_app.selected_transcode_key" :transcode_key="shared_app.selected_transcode_key"/>
<div class="w-full overflow-x-auto">
  <table class="table table-pin-rows table-extra-compact w-full">
    <colgroup>
      <col class="w-px"/>
      <col class="w-px"/>
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
            <div>Video ID</div>
            <div @click="click_sort_column('video_id')">
              <SortIcon :is_descending="get_sort_icon_mode('video_id')"/>
            </div>
          </div>
        </th>
        <th>
          <div class="inline-flex gap-2">
            <div>Ext</div>
            <div @click="click_sort_column('audio_ext')">
              <SortIcon :is_descending="get_sort_icon_mode('audio_ext')"/>
            </div>
          </div>
        </th>
        <th>
          <div class="inline-flex gap-2">
            <div>Status</div>
            <div @click="click_sort_column('status')">
              <SortIcon :is_descending="get_sort_icon_mode('status')"/>
            </div>
          </div>
        </th>
        <th>
          <div class="inline-flex gap-2">
            <div>Time</div>
            <div @click="click_sort_column('time')">
              <SortIcon :is_descending="get_sort_icon_mode('time')"/>
            </div>
          </div>
        </th>
        <th>
          <div class="inline-flex gap-2">
            <div>Title</div>
            <div @click="click_sort_column('title')">
              <SortIcon :is_descending="get_sort_icon_mode('title')"/>
            </div>
          </div>
        </th>
        <th>Audio</th>
        <th>Stdout</th>
        <th>Stderr</th>
        <th>System</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      <template v-for="({ state: item, metadata }, index) in sorted_items" :key="index">
        <tr
          class="hover:bg-base-300 cursor-pointer"
          :class="get_selected_class(item)"
          @click="select_transcode(item)"
        >
          <th>{{ item.video_id }}</th>
          <td>{{ item.audio_ext }}</td>
          <td>{{ item.status }}</td>
          <td><span class="text-nowrap">{{ format_datetime(item.unix_time) }}</span></td>
          <td>
            <template v-if="metadata.value !== undefined">{{ metadata.value.snippet.title }}</template>
            <template v-else>...</template>
          </td>
          <td>
            <AudioPlayer v-if="item.audio_path" :url="create_data_url(item.audio_path)" :rounded_left="true" :rounded_right="true"/>
          </td>
          <td>
            <a v-if="item.stdout_log_path" class="btn btn-sm px-1" :href="create_data_url(item.stdout_log_path)">
              <FileTerminal class="size-5"/>
            </a>
          </td>
          <td>
            <a v-if="item.stderr_log_path" class="btn btn-sm px-1" :href="create_data_url(item.stderr_log_path)">
              <FileTerminal class="size-5"/>
            </a>
          </td>
          <td>
            <a v-if="item.system_log_path" class="btn btn-sm px-1" :href="create_data_url(item.system_log_path)">
              <FileTerminal class="size-5"/>
            </a>
          </td>
          <td>
            <button class="btn btn-error btn-sm px-1" @click.stop="delete_transcode(item)" :disabled="is_worker_running(item.status)">
              <Trash2 class="size-5"/>
            </button>
          </td>
        </tr>
      </template>
      <template v-if="sorted_items.length === 0">
        <td colspan="10" class="text-center"><span class="text-nowrap">No transcodes</span></td>
      </template>
    </tbody>
  </table>
</div>
</template>
