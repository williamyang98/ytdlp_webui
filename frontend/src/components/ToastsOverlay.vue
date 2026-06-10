<script lang="ts" setup>
import { use_toasts_store, type ToastType } from "../stores/toast.ts";

function toast_class(type?: ToastType): string {
  switch (type) {
    case undefined: return "";
    case "info":    return "alert-info";
    case "warning": return "alert-warning";
    case "success": return "alert-success";
    case "error":   return "alert-error";
  }
}

const toasts = use_toasts_store();
</script>

<template>
<div class="toast z-3 overflow-hidden max-h-[50vh]">
  <template v-for="toast in toasts.toasts" :key="toast.id">
    <div
      class="cursor-pointer alert" :class="toast_class(toast.type)"
      @click="toasts.remove_toast(toast.id)"
    >
      <span>{{ toast.message }}</span>
    </div>
  </template>
</div>
</template>

