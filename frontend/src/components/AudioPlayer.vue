<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { LinkIcon, PauseIcon, PlayIcon } from "lucide-vue-next";
import { convert_dhms_to_string, convert_seconds_to_dhms } from "../utility/format";

const props = defineProps<{
  url: string,
  hide_duration?: boolean,
  hide_seek_slider?: boolean,
  hide_link?: boolean,
  rounded_left?: boolean,
  rounded_right?: boolean,
}>();

type Element = "play_button" | "player" | "link";

const last_element = computed((): Element => {
  if (!props.hide_link) return "link";
  if (!props.hide_seek_slider) return "player";
  if (!props.hide_duration) return "player";
  return "play_button";
});

interface RoundedClasses {
  'rounded-l': boolean;
  'rounded-r': boolean;
  'border-l-0': boolean;
  'border-r-0': boolean;
}
function get_rounded_classes(elem: Element): RoundedClasses {
  return {
    'rounded-l': props.rounded_left && elem === "play_button",
    'rounded-r': props.rounded_right && last_element.value === elem,
    'border-l-0': elem !== "play_button" && elem !== "link",
    'border-r-0': last_element.value !== elem && elem !== "play_button",
  }
}

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
    if (Number.isFinite(elem.duration)) {
      total_duration.value = elem.duration;
    }
  });
  elem.addEventListener("durationchange", () => {
    if (Number.isFinite(elem.duration)) {
      total_duration.value = elem.duration;
    }
  });
  elem.addEventListener("timeupdate", () => {
    if (Number.isFinite(elem.currentTime)) {
      current_seek.value = elem.currentTime;
    }
  });
  elem.addEventListener("play", () => {
    is_playing.value = true;
  });
  elem.addEventListener("pause", () => {
    is_playing.value = false;
  });
});

// when loading on press start playing immediately
watch(is_loaded, (is_loaded) => {
  if (!is_loaded) return;
  const elem = audio_elem.value;
  if (elem === null) return;
  elem.play()
    .catch((error: unknown) => {
      if (elem.paused) return;
      console.error(error);
    });
});

function format_duration(seconds: number): string {
  seconds = Math.round(seconds);
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
  <button
    @click.stop="toggle"
    class="btn btn-sm px-1 rounded-none"
    :class="get_rounded_classes('play_button')"
    :disabled="!can_play && is_playing"
  >
    <PauseIcon v-if="is_playing" class="size-5"/>
    <PlayIcon v-else class="size-5"/>
  </button>
  <div
    v-if="!hide_duration || !hide_seek_slider"
    class="flex gap-2 items-center bg-base-100 border-y border-base-300 border-1 px-2 w-full"
    :class="get_rounded_classes('player')"
    @click.stop
  >
    <span v-if="!hide_duration" class="label">{{ format_duration(current_seek) }} / {{ format_duration(total_duration) }}</span>
    <input
      v-if="!hide_seek_slider"
      class="range range-xs min-w-[5rem] w-full" type="range"
      min="0" :max="total_duration" :value="current_seek" step="1"
      :disabled="!is_loaded"
      @input.stop="on_target_seek"
      @change.stop="on_finish_seek"
    />
  </div>
  <a
    v-if="!hide_link"
    class="btn btn-sm px-1 rounded-none"
    :class="get_rounded_classes('link')"
    target="_blank" :href="url" @click.stop
  >
    <LinkIcon class="size-5"/>
  </a>
</div>
</template>
