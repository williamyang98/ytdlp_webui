<script setup lang="ts">
import { type TranscodeKey, type FfmpegRow } from "../api/ytdlp_api_schema.ts";
import SortIcon from "./SortIcon.vue";
import TranscodeProgressBar from "./TranscodeProgressBar.vue";
import { FileMusic, FileTerminal, Trash2 } from 'lucide-vue-next';
import { format_datetime } from "../utility/format.ts";
import { get_data_url } from "../api/api.ts";
import { ref, computed } from "vue";
import { providers } from "../providers/providers.ts";
import { is_worker_running } from "../api/ytdlp_api_schema.ts";

const app = providers.app;

const sort_order = ref<Order>({
  column: "time",
  is_descending: true,
});

const transcode_state = computed(() => {
  if (app.selected_transcode_key === null) return null;
  const transcode_worker = app.get_transcode_worker(app.selected_transcode_key);
  return transcode_worker.state;
});

type Column = "video_id" | "audio_ext" | "status" | "time";
interface Order {
  column: Column,
  is_descending: boolean,
}

function select_transcode(row: FfmpegRow) {
  const key: TranscodeKey = {
    video_id: row.video_id,
    audio_ext: row.audio_ext,
  };
  app.select_transcode(key);
}

async function delete_transcode(row: FfmpegRow) {
  const key: TranscodeKey = {
    video_id: row.video_id,
    audio_ext: row.audio_ext,
  };
  await app.delete_transcode(key);
}

function get_selected_class(row: FfmpegRow): string {
  const key = app.selected_transcode_key;
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

const sorted_items = computed(() => {
  const column = sort_order.value.column;
  const is_descending = sort_order.value.is_descending;
  const items = [...app.transcodes];
  switch (column) {
    case "video_id": {
      items.sort((a, b) => a.video_id.localeCompare(b.video_id));
      break;
    }
    case "audio_ext": {
      items.sort((a, b) => a.audio_ext.localeCompare(b.audio_ext));
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
<TranscodeProgressBar v-if="transcode_state" :state="transcode_state"/>
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
          @click="select_transcode(item)"
        >
          <th>{{ item.video_id }}</th>
          <td>{{ item.audio_ext }}</td>
          <td>{{ item.status }}</td>
          <td>{{ format_datetime(item.unix_time) }}</td>
          <td>
            <a v-if="item.audio_path" class="btn btn-sm px-1" :href="get_data_url(item.audio_path)">
              <FileMusic class="size-5"/>
            </a>
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
            <button class="btn btn-error btn-sm px-1" @click.stop="delete_transcode(item)" :disabled="is_worker_running(item.status)">
              <Trash2 class="size-5"/>
            </button>
          </td>
        </tr>
      </template>
    </tbody>
  </table>
</div>
</template>
