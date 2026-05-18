<script setup lang="ts">
import { type YtdlpRow } from "../api/ytdlp_api_schema.ts";
import SortIcon from "./SortIcon.vue";
import { FileMusic, FileTerminal, Trash2 } from 'lucide-vue-next';
import { format_date } from "../utility/format.ts";
import { get_data_url } from "../api/api.ts";
import { ref, computed } from "vue";
import { providers } from "../providers/providers.ts";

const app = providers.app;

function select_download(row: YtdlpRow) {
  app.select_download(row.video_id);
}

async function delete_download(row: YtdlpRow) {
  await app.delete_download(row.video_id);
}

function get_selected_class(row: YtdlpRow): string {
  return row.video_id === app.selected_download_key ? "bg-base-300" : "";
}

type Column = "video_id" | "status" | "time";
interface Order {
  column: Column,
  is_descending: boolean,
}

const sort_order = ref<Order>({
  column: "time",
  is_descending: true,
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
<table class="table table-pin-rows table-compact" :class="$attrs.class">
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
        <td>{{ format_date(item.unix_time) }}</td>
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
          <button class="btn btn-error btn-sm px-1" @click.stop="delete_download(item)">
            <Trash2 class="size-5"/>
          </button>
        </td>
      </tr>
    </template>
  </tbody>
</table>
</template>
