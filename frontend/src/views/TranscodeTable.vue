<script setup lang="ts">
import { type TranscodeKey, type FfmpegRow } from "../api/ytdlp_api_schema.ts";
import SortIcon from "../utility/SortIcon.vue";
import { FileMusic, FileTerminal, Trash2 } from 'lucide-vue-next';
import { format_date } from "../utility/format.ts";
import { get_data_url } from "../api/api.ts";
import { ref, computed } from "vue";

const props = defineProps<{
  items: FfmpegRow[],
  selected?: TranscodeKey,
}>();

const emits = defineEmits<{
  select: [TranscodeKey],
  delete: [TranscodeKey],
}>();

function select_transcode(row: FfmpegRow) {
  const key: TranscodeKey = {
    video_id: row.video_id,
    audio_ext: row.audio_ext,
  };
  emits("select", key);
}

function delete_transcode(row: FfmpegRow) {
  const key: TranscodeKey = {
    video_id: row.video_id,
    audio_ext: row.audio_ext,
  };
  emits("delete", key);
}

function get_selected_class(row: FfmpegRow): string {
  if (props.selected === undefined) return "";
  const is_selected = row.video_id === props.selected.video_id && row.audio_ext === props.selected.audio_ext;
  return is_selected ? "bg-base-300" : "";
}

type Column = "video_id" | "audio_ext" | "status" | "time";
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
  const items = [...props.items];
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
          <button class="btn btn-error btn-sm px-1" @click="delete_transcode(item)">
            <Trash2 class="size-5"/>
          </button>
        </td>
      </tr>
    </template>
  </tbody>
</table>
</template>
