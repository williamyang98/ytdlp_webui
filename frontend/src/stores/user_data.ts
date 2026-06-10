import { defineStore } from "pinia";
import { customRef } from "vue";

export const use_user_data_store = defineStore("user_data", () => {
  const storage: Storage = localStorage;

  function create_boolean_entry(key: string, default_value: boolean) {
    const initial_value = storage.getItem(key);
    let value = default_value;
    if (initial_value !== null) {
      value = initial_value === "true";
    }
    return customRef((track, trigger) => {
      return {
        get() {
          track();
          return value;
        },
        set(new_value: boolean) {
          value = new_value;
          storage.setItem(key, String(value));
          trigger();
        },
      }
    });
  }

  const is_dark_mode = create_boolean_entry("is_dark_mode", false);
  return {
    is_dark_mode,
  }
});

export type UserDataStore = ReturnType<typeof use_user_data_store>;
