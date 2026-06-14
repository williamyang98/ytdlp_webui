import HomePage from './HomePage.vue'
import DownloadsPage from './DownloadsPage.vue'
import TranscodesPage from './TranscodesPage.vue'
import SettingsPage from './SettingsPage.vue';
import type { Component, FunctionalComponent } from 'vue'
import { DownloadIcon, HomeIcon, SettingsIcon, TrafficConeIcon } from 'lucide-vue-next';

export interface Route {
  name: string,
  path: string,
  page_component: Component,
  icon_component?: FunctionalComponent,
}

export const routes: Route[] = [
  { path: "/", name: "Home", page_component: HomePage, icon_component: HomeIcon },
  { path: "/downloads", name: "Downloads", page_component: DownloadsPage, icon_component: DownloadIcon },
  { path: "/transcodes", name: "Transcodes", page_component: TranscodesPage, icon_component: TrafficConeIcon },
  { path: "/settings", name: "Settings", page_component: SettingsPage, icon_component: SettingsIcon },
];
