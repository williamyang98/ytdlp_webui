<script setup lang="ts">
import { type TranscodeKey, type TranscodeState } from "../api/ytdlp_api_schema.ts";
import { get_transcode_key_hash, use_cached_api_store } from "../stores/cached_api.ts";
import { convert_dhms_to_string, convert_seconds_to_dhms } from "../utility/format.ts";
import { ref, computed, watch } from "vue";

const props = defineProps<{
  transcode_key: TranscodeKey,
}>();
const cached_api = use_cached_api_store();

const transcode_key = computed(() => props.transcode_key);
watch(transcode_key, (transcode_key) => {
  cached_api.start_transcode_background_worker(transcode_key);
}, {
  immediate: true,
});

const state = computed(() => {
  const hash = get_transcode_key_hash(transcode_key.value);
  const transcode_state = cached_api.transcode_state[hash];
  return transcode_state;
});

const width = computed((): number => {
  if (state.value === undefined) return 0;
  switch (state.value.worker_status) {
    case "finished": return 1;
    case "failed": return 1;
    case "queued": return 1;
    case "running": break;
  }
  let total_milliseconds = state.value.source_duration_milliseconds;
  let elapsed_milliseconds = state.value.transcode_duration_milliseconds;
  if (elapsed_milliseconds !== undefined && total_milliseconds !== undefined) {
    total_milliseconds = Math.max(total_milliseconds, 1);
    elapsed_milliseconds = Math.max(elapsed_milliseconds, 0);
    const progress = elapsed_milliseconds/total_milliseconds;
    return progress;
  }
  return 0;
});

const colour = computed((): string => {
  if (state.value === undefined) return "";
  switch (state.value.worker_status) {
    case "finished": return "bg-success";
    case "failed": return "bg-error";
    case "queued": return "bg-warning";
    case "running": return "bg-info";
    default: return "";
  }
});

const status = computed((): string => {
  if (state.value === undefined) return "No Transcode";
  switch (state.value.worker_status) {
    case "finished": return state.value.file_cached ? "Transcode Finished (cached)" : "Transcode Finished";
    case "failed": return "Transcode Failed";
    case "queued": return "Transcode Queued";
    case "running": break;
  }
  const total_milliseconds = state.value.source_duration_milliseconds;
  const elapsed_milliseconds = state.value.transcode_duration_milliseconds;
  if (elapsed_milliseconds !== undefined && total_milliseconds !== undefined) {
    const elapsed_dhms = convert_seconds_to_dhms(elapsed_milliseconds/1000);
    const total_dhms = convert_seconds_to_dhms(total_milliseconds/1000);
    const elapsed_string = convert_dhms_to_string(elapsed_dhms);
    const total_string = convert_dhms_to_string(total_dhms);
    return `Transcoding ${elapsed_string}/${total_string}`;
  }
  return "Transcoding";
});

const subtitle = ref<string | null>(null);
function update_subtitle(new_state: TranscodeState | undefined, old_state: TranscodeState | undefined): string | null {
  if (new_state === undefined) return "Waiting for transcode to be queued";
  if (new_state.file_cached) return null;
  switch (new_state.worker_status) {
    case "failed": return new_state.fail_reason || "Failed with unprovided reason";
    case "queued": return "Waiting for transcode to run";
    case "running": break;
    case "finished":  break;
  }

  // Bitrate doesn't tell us anything useful about progress
  // if (state.source_speed_bits !== undefined && state.transcode_speed_bits !== undefined) {
  //   const { value: speed_bits, prefix: speed_bits_unit } = convert_to_short_standard_prefix(state.source_speed_bits);
  //   const { value: transcode_bits, prefix: transcode_bits_unit } = convert_to_short_standard_prefix(state.transcode_speed_bits);
  // }

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

  // estimate eta given elapsed time and percentage
  if (new_state.transcode_duration_milliseconds === undefined || new_state.source_duration_milliseconds === undefined) {
    return null;
  }
  // progress values
  const elapsed_time_string = convert_dhms_to_string(convert_seconds_to_dhms(new_state.transcode_duration_milliseconds/1000));
  const total_time_string = convert_dhms_to_string(convert_seconds_to_dhms(new_state.source_duration_milliseconds/1000));
  const text_time_progress = `${elapsed_time_string}/${total_time_string}`;

  // Estimate speed of transcode in transcode_duration per second
  let measure_duration_start_seconds: number = 0;
  const measure_duration_end_seconds = new_state.transcode_duration_milliseconds/1000;
  let measure_time_start_seconds = new_state.start_time_unix.getTime()/1000;
  const measure_time_end_seconds = new_state.end_time_unix.getTime()/1000;
  // try to use most recent measurement for more accurate speed estimate
  if (old_state !== undefined && old_state.id === new_state.id) {
    if (old_state.transcode_duration_milliseconds !== undefined) {
      measure_duration_start_seconds = old_state.transcode_duration_milliseconds/1000;
      measure_time_start_seconds = old_state.end_time_unix.getTime()/1000;
    }
  }
  // estimate speed
  const measure_duration_span_seconds = measure_duration_end_seconds-measure_duration_start_seconds;
  const measure_time_span_seconds = measure_time_end_seconds-measure_time_start_seconds;
  const estimated_transcode_speed = (measure_time_span_seconds === 0) ? 0 : measure_duration_span_seconds / measure_time_span_seconds;
  const estimated_transcode_speed_string = convert_dhms_to_string(convert_seconds_to_dhms(estimated_transcode_speed));
  // calculate eta
  const remaining_transcode_duration_seconds = (new_state.source_duration_milliseconds-new_state.transcode_duration_milliseconds)/1000;
  const eta_seconds = (estimated_transcode_speed === 0) ? 0 : remaining_transcode_duration_seconds / estimated_transcode_speed;
  const eta_seconds_string = convert_dhms_to_string(convert_seconds_to_dhms(eta_seconds));

  const text = `${text_time_progress} @ ${estimated_transcode_speed_string}/s (ETA ${eta_seconds_string})`
  return text;
}

watch(state, (new_state, old_state) => {
  subtitle.value = update_subtitle(new_state, old_state);
});

const subtitle_colour = computed(() => state.value?.worker_status === "failed" ? "text-error-content" : "");
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
