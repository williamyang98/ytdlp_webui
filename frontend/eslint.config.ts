import { globalIgnores } from 'eslint/config'
import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript'
import pluginVue from 'eslint-plugin-vue'

// To allow more languages other than `ts` in `.vue` files, uncomment the following lines:
// import { configureVueProject } from '@vue/eslint-config-typescript'
// configureVueProject({ scriptLangs: ['ts', 'tsx'] })
// More info at https://github.com/vuejs/eslint-config-typescript/#advanced-setup

export default defineConfigWithVueTs(
  globalIgnores([
    '**/dist/**',
    '**/dist-ssr/**',
    '**/coverage/**',
    'src/wasm/build/**',
    'src/wasm/vendor/**',
  ]),
  vueTsConfigs.strictTypeChecked,
  pluginVue.configs['flat/essential'],
  {
    name: 'app/files-to-lint',
    files: ['**/*.{ts,mts,tsx,vue}'],
    rules: {
      "@typescript-eslint/no-unused-vars": ['error', {
        argsIgnorePattern: '^_',
        varsIgnorePattern: '^_',
      }],
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      "no-fallthrough": ["error", {
        allowEmptyCase: false,
        commentPattern: "@fallthrough",
        reportUnusedFallthroughComment: true,
      }],
      "@typescript-eslint/restrict-template-expressions": ['error', {
        allowBoolean: true,
        allowNullish: true,
        allowNumber: true,
      }],
      // use tsconfig noImplicitReturns
      "@typescript-eslint/consistent-return": "off",
      "@typescript-eslint/no-unnecessary-condition": ["error", {
        allowConstantLoopConditions: "only-allowed-literals",
      }],
    },
  },
)
