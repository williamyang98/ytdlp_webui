import { defineStore } from "pinia";
import { ref } from "vue";

export type ToastType = "info" |  "success" | "warning" | "error";

export interface Toast {
  id: number;
  message: string;
  type?: ToastType;
}

export type ToastTimeout = number | null;

export const use_toasts_store = defineStore("toasts", () => {
  const toasts = ref<Toast[]>([]);
  const default_timeout_ms = ref<number>(5000);
  let head_id = 0;

  function push_toast(message: string, type?: ToastType): number {
    const id = head_id++;
    toasts.value.push({ id, message, type });
    return id;
  }

  function remove_toast(id: number): boolean {
    const index = toasts.value.findIndex(toast => toast.id === id);
    if (index < 0) return false;
    toasts.value.splice(index, 1);
    return true;
  }

  function create(message: string, type: ToastType | undefined, timeout_ms?: ToastTimeout) {
    const id = push_toast(message, type);
    if (timeout_ms === null) return;
    timeout_ms = timeout_ms ?? default_timeout_ms.value;
    setTimeout(() => {
      remove_toast(id);
    }, timeout_ms);
  }

  function log(message: string, timeout_ms?: ToastTimeout) { create(message, undefined, timeout_ms); }
  function error(message: string, timeout_ms?: ToastTimeout) { create(message, "error", timeout_ms); }
  function info(message: string, timeout_ms?: ToastTimeout) { create(message, "info", timeout_ms); }
  function warning(message: string, timeout_ms?: ToastTimeout) { create(message, "warning", timeout_ms); }
  function success(message: string, timeout_ms?: ToastTimeout) { create(message, "success", timeout_ms); }

  return {
    // state
    toasts,
    default_timeout_ms,
    // actions
    remove_toast,
    log,
    error,
    info,
    warning,
    success,
  }
});
