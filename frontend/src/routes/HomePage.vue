<script setup lang="ts">
import MetadataTable from "../components/MetadataTable.vue";
import RequestBar from "../components/RequestBar.vue";
import DownloadLink from "../components/DownloadLink.vue";
import { providers } from "../providers/providers.ts";

const app = providers.app;
</script>

<template>
<div class="w-full mt-2">
  <RequestBar/>
</div>
<div v-if="app.pending_request !== null" class="w-full">
  <div class="divider my-1"></div>
  <DownloadLink :pending_request="app.pending_request"/>
</div>
<div v-if="app.metadata !== null" class="w-full">
  <div class="divider my-1"></div>
  <MetadataTable/>
</div>
</template>
