<script setup lang="ts">
import UserDataProvider from "./Provider.vue";
import { MenuIcon, ChevronDownIcon, DownloadIcon } from 'lucide-vue-next';
import GithubIcon from "./github.svg";
import DarkModeToggle from "./DarkModeToggle.vue";
import { ref } from "vue";

import * as api from "./api.ts";
import { unix_time_to_string } from "./utility.ts";

const items = ref<api.FfmpegRow[]>([]);

async function get_transcodes() {
  const response = await api.get_transcodes();
  items.value = response;
}

</script>

<template>
<UserDataProvider>
<div class="w-screen h-screen overflow-hidden flex flex-col">
  <!-- Navbar -->
  <div class="navbar bg-base-100 shadow-sm min-h-[3rem] p-1">
    <div class="navbar-start w-full md:w-[50%]">
      <!--Mobile hamburger navigation menu-->
      <div class="dropdown lg:hidden">
        <div tabindex="0" role="button" class="btn btn-ghost py-1 px-2">
          <MenuIcon class="w-[1.5rem] h-[1.5rem]"/>
        </div>
        <ul tabindex="0" class="menu dropdown-content bg-base-100 rounded-box z-10 mt-3 min-w-52 p-2 shadow">
        </ul>
      </div>
      <!--Title-->
      <div class="app-title flex flex-row items-center gap-x-2 mx-2">
        <img src="/favicon.png" class="w-[2rem] h-[2rem] flex-none"/>
        <div class="text-lg text-nowrap font-medium">Ytdlp Webui</div>
      </div>
    </div>
    <!--Desktop navigation menu-->
    <div class="navbar-center hidden lg:flex">
      <ul class="menu menu-horizontal px-1 gap-x-1 z-10 p-0">
      </ul>
    </div>
    <div class="navbar-end gap-x-2">
      <a class="cursor-pointer mx-1" href="https://github.com/williamyang98/ytdlp_webui">
        <GithubIcon class="w-[1.75rem] h-[1.75rem]" style="fill: var(--color-base-content)"/>
      </a>
      <DarkModeToggle/>
    </div>
  </div>
  <div class="p-1 flex-1 min-h-0 w-full">
    Hello there

    <button class="btn btn-sm" @click="get_transcodes()">Download</button>
    <table class="table table-pin-rows table-compact" :class="$attrs.class">
      <thead>
        <tr>
          <th>Id</th>
          <th>Ext</th>
          <th>Status</th>
          <th>Time</th>
          <th>Stdout</th>
          <th>Stderr</th>
          <th>System</th>
          <th>Audio</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(item, index) in items" :key="index">
          <td class="font-medium text-nowrap">{{ item.video_id }}</td>
          <td>{{ item.audio_ext }}</td>
          <td>{{ item.status }}</td>
          <td>{{ unix_time_to_string(item.unix_time) }}</td>
          <td><a v-if="item.stdout_log_path" class="link link-primary" :href="api.get_data_url(item.stdout_log_path)">Link</a></td>
          <td><a v-if="item.stderr_log_path" class="link link-primary" :href="api.get_data_url(item.stderr_log_path)">Link</a></td>
          <td><a v-if="item.system_log_path" class="link link-primary" :href="api.get_data_url(item.system_log_path)">Link</a></td>
          <td><a v-if="item.audio_path" class="link link-primary" :href="api.get_data_url(item.audio_path)">Link</a></td>
        </tr>
      </tbody>
    </table>

  </div>
</div>
</UserDataProvider>
</template>

<style scoped>
@media (width < 24rem) {
.app-title {
  display: none;
}
}
</style>
