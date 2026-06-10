import './main.css'
import { createApp } from 'vue'
import { createRouter, createWebHashHistory } from 'vue-router'
import { createPinia } from 'pinia'
import MainView from './MainView.vue'
import { routes } from "./routes/routes.ts";

const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes: routes.map(route => {
    return { path: route.path, name: route.name, component: route.page_component };
  }),
});

const pinia = createPinia();
const app = createApp(MainView);
app.use(router);
app.use(pinia);
app.mount('#app');
