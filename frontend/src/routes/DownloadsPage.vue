<script setup lang="ts">
import DownloadTable from "../components/DownloadTable.vue";
import { providers } from "../providers/providers.ts";

const app = providers.app;
</script>

<template>
<div class="inline-flex w-full justify-between py-1">
  <h1 class="text-xl font-bold">Downloads</h1>
  <button class="btn btn-sm" @click="app.get_downloads()">Refresh</button>
</div>
<DownloadTable/>
</template>
