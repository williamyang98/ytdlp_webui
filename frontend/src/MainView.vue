<script setup lang="ts">
import { MenuIcon } from 'lucide-vue-next';
import UserDataProvider from "./providers/UserDataProvider.vue";
import AppProvider from "./providers/AppProvider.vue";
import GithubIcon from "./assets/github.svg";
import DarkModeToggle from "./components/DarkModeToggle.vue";
import AppView from "./AppView.vue";
</script>

<template>
<AppProvider>
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
  <!-- Body -->
  <div class="p-1 flex-1 w-full overflow-auto">
    <AppView/>
  </div>
</div>
</UserDataProvider>
</AppProvider>
</template>

<style scoped>
@media (width < 24rem) {
.app-title {
  display: none;
}
}
</style>
