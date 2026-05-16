import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueDevTools from 'vite-plugin-vue-devtools'
import tailwindcss from '@tailwindcss/vite';
import svgLoader from 'vite-svg-loader';

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd()) as Partial<Record<string, string>>;
  return {
    plugins: [
      vue(),
      vueDevTools(),
      tailwindcss(),
      svgLoader(),
    ],
    base: env.VITE_BASE_URL,
  }
})
