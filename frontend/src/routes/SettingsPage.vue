<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { use_cached_api_store } from '../stores/cached_api';
import { use_toasts_store } from '../stores/toast';


const cached_api = use_cached_api_store();
const toast = use_toasts_store();

onMounted(() => {
  if (cached_api.ytdlp_version === null) {
    void cached_api.get_ytdlp_version();
  }
});

const is_ytdlp_busy = ref(false);
async function get_ytdlp_version() {
  try {
    is_ytdlp_busy.value = true;
    void await cached_api.get_ytdlp_version(true);
  } catch (error: unknown) {
    toast.error(String(error));
  } finally {
    is_ytdlp_busy.value = false;
  }
}

async function request_ytdlp_update() {
  try {
    is_ytdlp_busy.value = true;
    void await cached_api.request_ytdlp_update(true);
    toast.success("Successfully updated ytdlp");
    void cached_api.get_ytdlp_version();
  } catch (error: unknown) {
    toast.error(String(error));
  } finally {
    is_ytdlp_busy.value = false;
  }
}

</script>

<template>
<div class="flex justify-between">
  <h1 class="text-xl font-bold">ytdlp application</h1>
  <div class="flex gap-x-0">
    <button class="btn btn-sm rounded-none rounded-l border-r-0" @click.stop="get_ytdlp_version" :disabled="is_ytdlp_busy">Refresh</button>
    <button class="btn btn-sm rounded-none rounded-r" @click.stop="request_ytdlp_update" :disabled="is_ytdlp_busy">Update</button>
  </div>
</div>
<div class="flex gap-x-1">
  <span class="font-medium">Version: </span>
  <span>{{ cached_api.ytdlp_version ?? '?' }}</span>
</div>
<div class="w-full border border-1 px-2" v-if="cached_api.ytdlp_update_log !== null">
  <code class="whitespace-pre-wrap">{{ cached_api.ytdlp_update_log }}</code>
</div>

</template>
