<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRouter, useRoute, RouterView } from 'vue-router';
import { routes } from "./routes/routes.ts";
import GithubIcon from "./assets/github.svg";
import DarkModeToggle from "./components/DarkModeToggle.vue";
import { MenuIcon } from 'lucide-vue-next';
// providers
import UserDataProvider from "./providers/UserDataProvider.vue";
import { use_cached_api_store } from "./stores/cached_api.ts";
import ToastsOverlay from "./components/ToastsOverlay.vue";

const router = useRouter();
const current_route = useRoute();
const cached_api = use_cached_api_store();

onMounted(() => {
  void cached_api.get_downloads(true);
  void cached_api.get_transcodes(true);
});

// close open dropdowns on route change (such as those on navbar)
watch(() => current_route.fullPath, () => {
  if (document.activeElement) {
    (document.activeElement as HTMLElement).blur();
  }
});

watch(() => current_route.name, (name) => {
  const route_name = name?.toString();
  if (route_name === undefined) return;
  document.title = `ytdlp - ${route_name}`;
});

</script>

<template>
<UserDataProvider>
<ToastsOverlay/>
<div class="w-screen h-screen overflow-hidden flex flex-col">
  <!-- Navbar -->
  <div class="navbar bg-base-100 shadow-sm min-h-[3rem] p-1">
    <div class="navbar-start w-full sm:w-[50%]">
      <!--Mobile hamburger navigation menu-->
      <div class="dropdown sm:hidden">
        <div tabindex="0" role="button" class="btn btn-ghost py-1 px-2">
          <MenuIcon class="w-[1.5rem] h-[1.5rem]"/>
        </div>
        <ul tabindex="0" class="menu dropdown-content bg-base-100 rounded-box z-10 mt-3 min-w-52 p-2 shadow">
          <li v-for="route of routes" :key="route.name">
            <a :href="router.resolve(route.path).href" class="w-full group" :class="{ 'menu-active': route.name === current_route.name }">
              <component v-if="route.icon_component" :is="route.icon_component" class="size-5"/>
              <span class="whitespace-nowrap">{{ route.name }}</span>
            </a>
          </li>
        </ul>
      </div>
      <!--Title-->
      <div class="app-title flex flex-row items-center gap-x-2 mx-2">
        <img src="/favicon.png" class="w-[2rem] h-[2rem] flex-none"/>
        <div class="text-lg text-nowrap font-medium">Ytdlp Webui</div>
      </div>
    </div>
    <!--Desktop navigation menu-->
    <div class="navbar-center hidden sm:flex">
      <ul class="menu menu-horizontal px-1 gap-x-1 z-10 p-0">
        <li v-for="route of routes" :key="route.name">
          <a :href="router.resolve(route.path).href" class="w-full flex flex-row gap-x-2 items-center" :class="{ 'menu-active': route.name === current_route.name }">
            <component v-if="route.icon_component" :is="route.icon_component" class="size-5"/>
            <span class="whitespace-nowrap">{{ route.name }}</span>
          </a>
        </li>
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
  <div class="p-1 flex-1 w-full overflow-y-auto overflow-x-hidden">
    <RouterView/>
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
