<script setup lang="ts">
import TranscodeTable from "../components/TranscodeTable.vue";
import { providers } from "../providers/providers.ts";

const app = providers.app;
</script>

<template>
<div class="inline-flex w-full justify-between py-1">
  <h1 class="text-xl font-bold">Transcodes</h1>
  <button class="btn btn-sm" @click="app.get_transcodes()">Refresh</button>
</div>
<TranscodeTable/>
</template>
