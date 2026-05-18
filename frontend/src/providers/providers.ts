import { inject, type Ref, type Reactive } from "vue";
import { ToastManager } from "./toast.ts";
import { UserData } from "./user_data.ts";
import { App } from "./app.ts";

export const providers = {
  get toast_manager(): Ref<ToastManager> {
    const value = inject<Ref<ToastManager>>("toast_manager");
    if (value === undefined) throw Error("Expected toast_manager to be injected from provider");
    return value;
  },
  get user_data(): Ref<UserData> {
    const value = inject<Ref<UserData | undefined>>("user_data");
    if (value === undefined) throw Error("Expected user_data to be injected from provider");
    return value as Ref<UserData>;
  },
  get app(): Reactive<App> {
    const value = inject<Reactive<App>>("app");
    if (value === undefined) throw Error("Expected app to be injected from provider");
    return value;
  },
}
