<script setup lang="ts">
import { type TranscodeState } from "../api/ytdlp_api_schema.ts";
import { convert_dhms_to_string, convert_seconds_to_dhms } from "../utility/format.ts";
import { computed } from "vue";

const props = defineProps<{
  state: TranscodeState | null,
}>();

const width = computed((): number => {
  const state = props.state;
  if (state === null) return 0;
  switch (state.worker_status) {
    case "finished": return 1;
    case "failed": return 1;
    case "queued": return 0;
    case "running": break;
  }
  let total_milliseconds = state.source_duration_milliseconds;
  let elapsed_milliseconds = state.transcode_duration_milliseconds;
  if (elapsed_milliseconds !== undefined && total_milliseconds !== undefined) {
    total_milliseconds = Math.max(total_milliseconds, 1);
    elapsed_milliseconds = Math.max(elapsed_milliseconds, 0);
    const progress = elapsed_milliseconds/total_milliseconds;
    return progress;
  }
  return 0;
});

const colour = computed((): string => {
  const state = props.state;
  if (state === null) return "Pending";
  switch (state.worker_status) {
    case "finished": return "bg-green-400";
    case "failed": return "bg-red-400";
    case "queued": return "bg-orange-400";
    case "running": return "bg-blue-400";
    default: return "";
  }
});

const status = computed((): string => {
  const state = props.state;
  if (state === null) return "Waiting";
  switch (state.worker_status) {
    case "finished": return state.file_cached ? "Finished (cached)" : "Finished";
    case "failed": return "Failed";
    case "queued": return "Queued";
    case "running": break;
  }
  const total_milliseconds = state.source_duration_milliseconds;
  const elapsed_milliseconds = state.transcode_duration_milliseconds;
  if (elapsed_milliseconds !== undefined && total_milliseconds !== undefined) {
    const elapsed_dhms = convert_seconds_to_dhms(elapsed_milliseconds/1000);
    const total_dhms = convert_seconds_to_dhms(total_milliseconds/1000);
    const elapsed_string = convert_dhms_to_string(elapsed_dhms);
    const total_string = convert_dhms_to_string(total_dhms);
    return `${elapsed_string}/${total_string}`;
  }
  return "Running";
});

const subtitle = computed((): string | null => {
  const state = props.state;
  if (state === null) return "Waiting for transcode to be queued";
  if (state.file_cached) return null;
  switch (state.worker_status) {
    case "failed": return state.fail_reason || "Failed with unprovided reason";
    case "queued": return "Waiting for transcode to run";
    case "running": break;
    case "finished":  break;
  }

  // Bitrate doesn't tell us anything useful about progress
  // if (state.source_speed_bits !== undefined && state.transcode_speed_bits !== undefined) {
  //   const { value: speed_bits, prefix: speed_bits_unit } = convert_to_short_standard_prefix(state.source_speed_bits);
  //   const { value: transcode_bits, prefix: transcode_bits_unit } = convert_to_short_standard_prefix(state.transcode_speed_bits);
  // }

  // estimate eta given elapsed time and percentage
  const time_elapsed_milliseconds = state.end_time_unix.getTime() - state.start_time_unix.getTime();
  const time_elapsed_seconds = time_elapsed_milliseconds/1000;
  if (state.transcode_duration_milliseconds === undefined || state.source_duration_milliseconds === undefined) {
    return null;
  }
  const percentage = state.transcode_duration_milliseconds / Math.max(state.source_duration_milliseconds, 1);
  const remaining_percentage = 1 - percentage;
  const eta_seconds = (time_elapsed_seconds/percentage)*remaining_percentage;

  const eta_seconds_string = convert_dhms_to_string(convert_seconds_to_dhms(eta_seconds));
  const elapsed_time_string = convert_dhms_to_string(convert_seconds_to_dhms(state.transcode_duration_milliseconds/1000));
  const total_time_string = convert_dhms_to_string(convert_seconds_to_dhms(state.source_duration_milliseconds/1000));
  const text_time_progress = `${elapsed_time_string}/${total_time_string}`;

  // This is now garbage because the transcode size gives erroneous values when embedding thumbnail
  // if (state.transcode_size_bytes !== undefined) {
  //   // estimate size of final file
  //   const estimated_total_bytes = state.transcode_size_bytes/percentage;
  //   const { value: elapsed_bytes, prefix: elapsed_bytes_unit } = convert_to_short_standard_prefix(state.transcode_size_bytes);
  //   const { value: total_bytes, prefix: total_bytes_unit } = convert_to_short_standard_prefix(estimated_total_bytes);
  //   // estimate speed of transcode
  //   const estimated_speed_bytes = (time_elapsed_seconds === 0) ? 0 : state.transcode_size_bytes / time_elapsed_seconds;
  //   const { value: speed_bytes, prefix: speed_bytes_unit } = convert_to_short_standard_prefix(estimated_speed_bytes);
  //   // create subtitle
  //   const text_size_progress = `${elapsed_bytes.toFixed(2)}${elapsed_bytes_unit}B/${total_bytes.toFixed(2)}${total_bytes_unit}B`;
  //   const text_speed = `${speed_bytes.toFixed(2)}${speed_bytes_unit}B/s`;
  //   const text = `${text_time_progress} - ${text_size_progress} @ ${text_speed} (${eta_string})`
  //   return text;
  // }

  // Estimate speed of transcode in transcode_duration per second
  const transcode_duration_seconds = state.transcode_duration_milliseconds/1000;
  const estimated_transcode_speed = (time_elapsed_seconds === 0) ? 0 : transcode_duration_seconds / time_elapsed_seconds;
  const estimated_transcode_speed_string = convert_dhms_to_string(convert_seconds_to_dhms(estimated_transcode_speed));
  const text = `${text_time_progress} @ ${estimated_transcode_speed_string}/s (ETA ${eta_seconds_string})`
  return text;
});
</script>

<template>
<div class="w-full">
  <div class="rounded-sm w-full h-[2.0rem] bg-slate-300 border-1 border-slate-300 border-sm">
    <div
      class="rounded-sm h-full text-center ease-width"
      :class="`${colour}`"
      :style="{ width: `${(width*100).toFixed(2)}%` }"
    >
      <span class="align-middle px-2 font-medium">{{ status }}</span>
    </div>
  </div>
  <p v-if="subtitle !== null" class="label">{{ subtitle }}</p>
</div>
</template>

<style scoped>
.ease-width {
  transition: width 0.2s ease;
}
</style>
