<script setup lang="ts">
import SortIcon from "./SortIcon.vue";
import DownloadProgressBar from "./DownloadProgressBar.vue";
import { ref, computed, type ComputedRef, useTemplateRef } from "vue";
import { FileTerminal, OctagonAlertIcon, RefreshCwIcon, Trash2, TrashIcon } from 'lucide-vue-next';
import AudioPlayer from "./AudioPlayer.vue";

import { type YtdlpRow } from "../api/ytdlp_api_schema.ts";
import { format_datetime } from "../utility/format.ts";
import { create_data_url } from "../api/api.ts";
import { is_worker_running } from "../api/ytdlp_api_schema.ts";
import { use_cached_api_store } from "../stores/cached_api.ts";
import { use_shared_app_store } from "../stores/shared_app.ts";
import { type VideoItem } from "../api/youtube_api_schema.ts";

const cached_api = use_cached_api_store();
const shared_app = use_shared_app_store();

const sort_order = ref<Order>({
  column: "time",
  is_descending: true,
});

type Column = "video_id" | "status" | "time" | "title";
interface Order {
  column: Column,
  is_descending: boolean,
}

function select_download(row: YtdlpRow) {
  shared_app.select_download(row.video_id);
}

async function delete_download(row: YtdlpRow) {
  await cached_api.delete_download(row.video_id);
}

function get_selected_class(row: YtdlpRow): string {
  return row.video_id === shared_app.selected_download_key ? "bg-base-300" : "";
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
  state: YtdlpRow;
  metadata: ComputedRef<VideoItem | undefined>;

  constructor(state: YtdlpRow) {
    const video_id = state.video_id;
    this.state = state;
    this.metadata = computed(() => {
      return cached_api.youtube_videos[video_id];
    });
    void cached_api.get_youtube_video(video_id);
  }
}

const items = computed(() => {
  return Object.values(cached_api.downloads).filter(v => v !== undefined).map(v => new Row(v));
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

const delete_modal = useTemplateRef("delete-modal");
function delete_all_downloads() {
  const video_ids = sorted_items.value.map(v => v.state.video_id);
  const promises = video_ids.map(id => cached_api.delete_download(id));
  void promises;
  delete_modal.value?.close();
}

</script>

<template>
<div class="inline-flex w-full justify-between py-1">
  <h1 class="text-xl font-bold">Downloads ({{ sorted_items.length }})</h1>
  <div class="flex">
    <button class="btn btn-sm px-1 rounded-none rounded-l" @click="delete_modal?.showModal()"><TrashIcon class="size-5"/></button>
    <button class="btn btn-sm px-1 rounded-none rounded-r" @click="cached_api.get_downloads(true)"><RefreshCwIcon class="size-5"/></button>
  </div>
</div>
<dialog class="modal" ref="delete-modal">
  <div class="modal-box justify-items-center">
    <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click.stop="delete_modal?.close()">✕</button>
    <OctagonAlertIcon class="size-[5rem] text-error"/>
    <h3 class="text-lg font-bold pt-2">Delete all downloads</h3>
    <p class="text-center pt-2">Are you sure you want to delete all downloads?<br>This action cannot be undone</p>
    <div class="flex w-full justify-between pt-2">
      <button class="btn" @click.stop="delete_modal?.close()">Cancel</button>
      <button class="btn btn-error" @click.stop="delete_all_downloads">Delete All</button>
    </div>
  </div>
  <div class="modal-backdrop" @click.stop="delete_modal?.close()"></div>
</dialog>
<DownloadProgressBar v-if="shared_app.selected_download_key !== null" :download_key="shared_app.selected_download_key"/>
<div class="w-full overflow-x-auto">
  <table class="table table-pin-rows table-extra-compact w-full">
    <colgroup>
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
          @click="select_download(item)"
        >
          <th>{{ item.video_id }}</th>
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
            <button class="btn btn-error btn-sm px-1" @click.stop="delete_download(item)" :disabled="is_worker_running(item.status)">
              <Trash2 class="size-5"/>
            </button>
          </td>
        </tr>
      </template>
      <template v-if="sorted_items.length === 0">
        <td colspan="9" class="text-center"><span class="text-nowrap">No downloads</span></td>
      </template>
    </tbody>
  </table>
</div>
</template>
