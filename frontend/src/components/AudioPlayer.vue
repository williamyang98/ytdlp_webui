<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { LinkIcon, PauseIcon, PlayIcon } from "lucide-vue-next";
import { convert_dhms_to_string, convert_seconds_to_dhms } from "../utility/format";

const props = defineProps<{
  url: string,
}>();

const audio_elem = ref<HTMLAudioElement | null>(null);
const can_play = ref(false);
const is_loaded = ref(false);
const is_playing = ref(false);
const current_seek = ref(0);
const total_duration = ref(0);


function toggle() {
  const elem = audio_elem.value;
  if (elem === null) {
    audio_elem.value = new Audio(props.url);
    audio_elem.value.load();
    return;
  }

  if (is_playing.value) {
    elem.pause();
    return;
  }

  if (!is_loaded.value) {
    elem.load();
  }
  elem.play()
    .catch((error: unknown) => {
      if (elem.paused) return;
      console.error(error);
    });
}

watch(audio_elem, (elem) => {
  if (elem === null) return;
  elem.volume = 1.0;
  elem.muted = false;
  elem.addEventListener("canplay", () => {
    can_play.value = true;
  });
  elem.addEventListener("load", () => {
    is_loaded.value = true;
  });
  elem.addEventListener("loadstart", () => {
    is_loaded.value = true;
  });
  elem.addEventListener("loadeddata", () => {
    is_loaded.value = true;
    total_duration.value = elem.duration;
  });
  elem.addEventListener("durationchange", () => {
    total_duration.value = elem.duration;
  });
  elem.addEventListener("timeupdate", () => {
    current_seek.value = elem.currentTime;
  });
  elem.addEventListener("play", () => {
    is_playing.value = true;
  });
  elem.addEventListener("pause", () => {
    is_playing.value = false;
  });
});

function format_duration(seconds: number): string {
  const dhms = convert_seconds_to_dhms(seconds);
  return convert_dhms_to_string(dhms);
}

function on_target_seek(ev: InputEvent) {
  if (ev.target === null) return;
  const input_elem = ev.target as HTMLInputElement;
  const target_seek = input_elem.valueAsNumber;
  if (Number.isNaN(target_seek)) return;

  if (audio_elem.value === null) return;
  if (!audio_elem.value.paused) audio_elem.value.pause();
  audio_elem.value.currentTime = target_seek;
}

function on_finish_seek() {
  void audio_elem.value?.play();
}

onBeforeUnmount(() => {
  const elem = audio_elem.value;
  if (elem === null) return;
  elem.pause();
});

</script>

<template>
<div class="flex w-full">
  <button @click.stop="toggle" class="btn btn-sm px-1 rounded-none rounded-l" :disabled="!can_play && is_playing">
    <PauseIcon v-if="is_playing" class="size-5"/>
    <PlayIcon v-else class="size-5"/>
  </button>
  <div class="flex gap-2 items-center bg-base-100 border-y border-base-300 px-2 w-full" @click.stop>
    <span class="label">{{ format_duration(current_seek) }} / {{ format_duration(total_duration) }}</span>
    <input
      class="range range-xs min-w-[5rem] w-full" type="range"
      min="0" :max="total_duration" :value="current_seek" step="1"
      :disabled="!is_loaded"
      @input.stop="on_target_seek"
      @change.stop="on_finish_seek"
    />
  </div>
  <a class="btn btn-sm px-1 rounded-none rounded-r" target="_blank" :href="url" @click.stop>
    <LinkIcon class="size-5"/>
  </a>
</div>
</template>
