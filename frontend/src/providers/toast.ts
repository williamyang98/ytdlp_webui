export type ToastType = "info" |  "success" | "warning" | "error";

export interface Toast {
  id: number;
  message: string;
  type?: ToastType;
}

export type ToastTimeout = number | undefined | null;

export class ToastManager {
  head_id: number = 0;
  toasts: Toast[] = [];
  default_timeout_ms: number = 5000;
  push_toast(message: string, type?: ToastType): number {
    const id = this.head_id++;
    this.toasts.push({ id, message, type });
    return id;
  }
  remove_toast(id: number): boolean {
    const index = this.toasts.findIndex(toast => toast.id === id);
    if (index < 0) return false;
    this.toasts.splice(index, 1);
    return true;
  }
  create(message: string, type: ToastType | undefined, timeout_ms: ToastTimeout) {
    const id = this.push_toast(message, type);
    if (timeout_ms === null) return;
    timeout_ms = timeout_ms ?? this.default_timeout_ms;
    setTimeout(() => {
      this.remove_toast(id);
    }, timeout_ms);
  }
  log(message: string, timeout_ms?: ToastTimeout) { this.create(message, undefined, timeout_ms); }
  error(message: string, timeout_ms?: ToastTimeout) { this.create(message, "error", timeout_ms); }
  info(message: string, timeout_ms?: ToastTimeout) { this.create(message, "info", timeout_ms); }
  warning(message: string, timeout_ms?: ToastTimeout) { this.create(message, "warning", timeout_ms); }
  success(message: string, timeout_ms?: ToastTimeout) { this.create(message, "success", timeout_ms); }
}
