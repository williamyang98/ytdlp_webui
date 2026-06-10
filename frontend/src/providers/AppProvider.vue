<script setup lang="ts">
import { reactive, provide, onMounted } from "vue";
import { App } from "./app.ts";
import { use_cached_api_store } from "../stores/cached_api.ts";

const cached_api = use_cached_api_store();

const app = reactive(new App());
provide("app", app);

onMounted(() => {
  app.on_mount(cached_api);
  void cached_api.get_downloads(true);
  void cached_api.get_transcodes(true);
});
</script>

<template>
<slot></slot>
</template>
