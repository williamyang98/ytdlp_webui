<script setup lang="ts">
import { type DownloadState } from "../api/ytdlp_api_schema.ts";
import { convert_dhms_to_string, convert_seconds_to_dhms, convert_to_short_standard_prefix } from "../utility/format.ts";
import { computed } from "vue";

const props = defineProps<{
  state: DownloadState | null,
}>();

const width = computed((): number => {
  const state = props.state;
  if (state === null) return 0;
  switch (state.worker_status) {
    case "finished": return 1;
    case "failed": return 1;
    case "queued": return 1;
    case "running": break;
  }
  let total_bytes = state.total_bytes;
  let elapsed_bytes = state.downloaded_bytes;
  if (elapsed_bytes !== undefined && total_bytes !== undefined) {
    total_bytes = Math.max(total_bytes, 1);
    elapsed_bytes = Math.max(elapsed_bytes, 0);
    const progress = elapsed_bytes/total_bytes;
    return progress;
  }
  return 0;
});

const colour = computed((): string => {
  const state = props.state;
  if (state === null) return "";
  switch (state.worker_status) {
    case "finished": return "bg-success";
    case "failed": return "bg-error";
    case "queued": return "bg-warning";
    case "running": return "bg-info";
    default: return "";
  }
});

const status = computed((): string => {
  const state = props.state;
  if (state === null) return "No Download";
  switch (state.worker_status) {
    case "finished": return state.file_cached ? "Download Finished (cached)" : "Download Finished";
    case "failed": return "Download Failed";
    case "queued": return "Download Queued";
    case "running": break;
  }
  let total_bytes = state.total_bytes;
  let elapsed_bytes = state.downloaded_bytes;
  if (elapsed_bytes !== undefined && total_bytes !== undefined) {
    total_bytes = Math.max(total_bytes, 0);
    elapsed_bytes = Math.max(elapsed_bytes, 0);
    const { value: elapsed_value, prefix: elapsed_suffix } = convert_to_short_standard_prefix(elapsed_bytes);
    const { value: total_value, prefix: total_suffix } = convert_to_short_standard_prefix(total_bytes);
    return `Downloading ${elapsed_value.toFixed(2)}${elapsed_suffix}B/${total_value.toFixed(2)}${total_suffix}B`;
  }
  return "Downloading";
});

const subtitle = computed((): string | null => {
  const state = props.state;
  if (state === null) return "Waiting for download to be queued";
  if (state.file_cached) return null;
  switch (state.worker_status) {
    case "failed": return state.fail_reason || "Failed with unprovided reason";
    case "queued": return "Waiting for download to run";
    case "running": break;
    case "finished":  break;
  }

  if (state.downloaded_bytes === undefined || state.total_bytes === undefined) {
    return null;
  }

  const { value: elapsed_bytes, prefix: elapsed_bytes_unit } = convert_to_short_standard_prefix(state.downloaded_bytes);
  const { value: total_bytes, prefix: total_bytes_unit } = convert_to_short_standard_prefix(state.total_bytes);

  let text_prediction = "";
  if (state.eta_seconds !== undefined && state.speed_bytes) {
    const { value: speed_bytes, prefix: speed_bytes_unit } = convert_to_short_standard_prefix(state.speed_bytes);
    const eta_string = convert_dhms_to_string(convert_seconds_to_dhms(state.eta_seconds));
    text_prediction = `@ ${speed_bytes.toFixed(2)}${speed_bytes_unit}B/s - (ETA ${eta_string})`;
  } else {
    text_prediction = "- (Unknown estimated time)";
  }
  const text_size_progress = `${elapsed_bytes.toFixed(2)}${elapsed_bytes_unit}B/${total_bytes.toFixed(2)}${total_bytes_unit}B`;
  const text = `${text_size_progress} ${text_prediction}`
  return text;
});

const subtitle_colour = computed(() => props.state?.worker_status === "failed" ? "text-error-content" : "");
</script>

<template>
<div class="w-full">
  <div class="rounded-sm w-full h-[2.0rem] bg-slate-300 border-1 border-slate-300 border-sm">
    <div
      class="rounded-sm h-full ease-width grid place-items-center"
      :class="`${colour}`"
      :style="{ width: `${(width*100).toFixed(2)}%` }"
    >
      <span class="text-center align-middle px-2 font-medium text-sm text-nowrap">{{ status }}</span>
    </div>
  </div>
  <p v-if="subtitle !== null" class="label text-sm text-nowrap px-1" :class="subtitle_colour">{{ subtitle }}</p>
</div>
</template>

<style scoped>
.ease-width {
  transition: width 0.2s ease;
}
</style>
