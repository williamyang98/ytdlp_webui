<script lang="ts" setup>
import { MoonIcon, SunIcon } from 'lucide-vue-next';
import { use_user_data_store } from '../stores/user_data';

const user_data = use_user_data_store();
</script>

<template>
<label class="flex flex-row items-center cursor-pointer gap-x-1">
  <SunIcon class="h-5 w-5"/>
  <input type="checkbox" v-model="user_data.is_dark_mode" class="toggle theme-controller" value="dark"/>
  <MoonIcon class="h-5 w-5"/>
</label>
</template>
