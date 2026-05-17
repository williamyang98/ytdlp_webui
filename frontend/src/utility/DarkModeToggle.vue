<script lang="ts" setup>
import { providers } from "../providers/providers.ts";
import { MoonIcon, SunIcon } from 'lucide-vue-next';
const user_data = providers.user_data.value;
</script>

<template>
<label class="flex flex-row items-center cursor-pointer gap-x-1">
  <SunIcon class="h-5 w-5"/>
  <input type="checkbox" v-model="user_data.is_dark_mode" class="toggle theme-controller" value="dark"/>
  <MoonIcon class="h-5 w-5"/>
</label>
</template>
