<script lang="ts" setup>
import { ref, provide } from "vue";
import { type ToastType, ToastManager } from "./toast.ts";

const manager = ref(new ToastManager());
provide("toast_manager", manager);

function toast_class(type?: ToastType): string {
  switch (type) {
    case undefined: return "";
    case "info":    return "alert-info";
    case "warning": return "alert-warning";
    case "success": return "alert-success";
    case "error":   return "alert-error";
  }
}

</script>

<template>
<div class="toast z-3 overflow-hidden max-h-[50vh]">
  <template v-for="toast in manager.toasts" :key="toast.id">
    <div
      class="cursor-pointer alert" :class="toast_class(toast.type)"
      @click="manager.remove_toast(toast.id)"
    >
      <span>{{ toast.message }}</span>
    </div>
  </template>
</div>
<slot></slot>
</template>
