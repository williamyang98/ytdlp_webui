<script setup lang="ts">
import SortIcon from "./SortIcon.vue";
import DownloadProgressBar from "./DownloadProgressBar.vue";
import { ref, computed } from "vue";
import { FileTerminal, Trash2 } from 'lucide-vue-next';
import AudioPlayer from "./AudioPlayer.vue";

import { type YtdlpRow } from "../api/ytdlp_api_schema.ts";
import { format_datetime } from "../utility/format.ts";
import { get_data_url } from "../api/api.ts";
import { providers } from "../providers/providers.ts";
import { is_worker_running } from "../api/ytdlp_api_schema.ts";

const app = providers.app;

const sort_order = ref<Order>({
  column: "time",
  is_descending: true,
});

const download_state = computed(() => {
  if (app.selected_download_key === null) return null;
  const download_worker = app.get_download_worker(app.selected_download_key);
  return download_worker.state;
});

type Column = "video_id" | "status" | "time";
interface Order {
  column: Column,
  is_descending: boolean,
}

function select_download(row: YtdlpRow) {
  app.select_download(row.video_id);
}

async function delete_download(row: YtdlpRow) {
  await app.delete_download(row.video_id);
}

function get_selected_class(row: YtdlpRow): string {
  return row.video_id === app.selected_download_key ? "bg-base-300" : "";
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

const sorted_items = computed(() => {
  const column = sort_order.value.column;
  const is_descending = sort_order.value.is_descending;
  const items = [...app.downloads];
  switch (column) {
    case "video_id": {
      items.sort((a, b) => a.video_id.localeCompare(b.video_id));
      break;
    }
    case "status": {
      items.sort((a, b) => a.status.localeCompare(b.status));
      break;
    }
    case "time": {
      items.sort((a, b) => a.unix_time.getTime()-b.unix_time.getTime());
      break;
    }
  }
  if (is_descending) {
    items.reverse();
  }
  return items;
});

</script>

<template>
<DownloadProgressBar v-if="download_state" :state="download_state"/>
<div class="w-full overflow-x-auto">
  <table class="table table-pin-rows table-extra-compact w-full">
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
        <th>Audio</th>
        <th>Stdout</th>
        <th>Stderr</th>
        <th>System</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      <template v-for="(item, index) in sorted_items" :key="index">
        <tr
          class="hover:bg-base-300 cursor-pointer"
          :class="get_selected_class(item)"
          @click="select_download(item)"
        >
          <th>{{ item.video_id }}</th>
          <td>{{ item.status }}</td>
          <td>{{ format_datetime(item.unix_time) }}</td>
          <td>
            <AudioPlayer v-if="item.audio_path" :url="get_data_url(item.audio_path)"/>
          </td>
          <td>
            <a v-if="item.stdout_log_path" class="btn btn-sm px-1" :href="get_data_url(item.stdout_log_path)">
              <FileTerminal class="size-5"/>
            </a>
          </td>
          <td>
            <a v-if="item.stderr_log_path" class="btn btn-sm px-1" :href="get_data_url(item.stderr_log_path)">
              <FileTerminal class="size-5"/>
            </a>
          </td>
          <td>
            <a v-if="item.system_log_path" class="btn btn-sm px-1" :href="get_data_url(item.system_log_path)">
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
        <td colspan="8" class="text-center"><span class="text-nowrap">No downloads</span></td>
      </template>
    </tbody>
  </table>
</div>
</template>
