<script setup lang="ts">
import { reactive, provide, onMounted } from "vue";
import { App } from "./app.ts";

const app = reactive(new App());
provide("app", app);


onMounted(() => {
  const _ = app.on_mount();
});
</script>

<template>
<slot></slot>
</template>
