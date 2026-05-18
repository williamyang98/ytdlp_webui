<script setup lang="ts">
import MetadataTable from "./components/MetadataTable.vue";
import TranscodeTable from "./components/TranscodeTable.vue";
import DownloadTable from "./components/DownloadTable.vue";
import RequestBar from "./components/RequestBar.vue";
import DownloadLink from "./components/DownloadLink.vue";
import { providers } from "./providers/providers.ts";

const app = providers.app;
</script>

<template>
<div>
  <h1 class="text-xl font-bold">Request</h1>
  <RequestBar/>
</div>
<br>
<template v-if="app.pending_request !== null">
  <h1 class="text-xl font-bold">Download Link</h1>
  <DownloadLink :pending_request="app.pending_request"/>
  <br>
</template>
<template v-if="app.metadata !== null">
  <h1 class="text-xl font-bold">Metadata</h1>
  <MetadataTable/>
  <br>
</template>
<div class="inline-flex w-full justify-between">
  <h1 class="text-xl font-bold">Downloads</h1>
  <button class="btn btn-sm" @click="app.get_downloads()">Refresh</button>
</div>
<DownloadTable/>
<br>
<div class="inline-flex w-full justify-between">
  <h1 class="text-xl font-bold">Transcodes</h1>
  <button class="btn btn-sm" @click="app.get_transcodes()">Refresh</button>
</div>
<TranscodeTable/>
</template>
