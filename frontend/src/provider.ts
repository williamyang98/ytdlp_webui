import { inject, type Ref } from "vue";

function try_into_boolean(storage: Storage, key: string, defualt_value: boolean): boolean {
  const value = storage.getItem(key);
  if (value === null) return defualt_value;
  return value === "true";
}

class BooleanEntry {
  storage: Storage;
  key: string;
  _value: boolean;

  constructor(storage: Storage, key: string, default_value: boolean) {
    this.storage = storage;
    this.key = key;
    this._value = try_into_boolean(storage, key, default_value);
  }

  get value(): boolean {
    return this._value;
  }

  set value(value: boolean) {
    this._value = value;
    this.storage.setItem(this.key, value ? "true" : "false");
  }
}

export class UserData {
  storage: Storage;
  _is_dark_mode: BooleanEntry;

  constructor(storage: Storage) {
    this.storage = storage;
    this._is_dark_mode = new BooleanEntry(storage, "is_dark_mode", false);
  }

  get is_dark_mode(): boolean { return this._is_dark_mode.value; }
  set is_dark_mode(value: boolean) { this._is_dark_mode.value = value; }
}

export const providers = {
  get user_data(): Ref<UserData> {
    const value = inject<Ref<UserData | undefined>>("user_data");
    if (value === undefined) throw Error("Expected user_data to be injected from provider");
    return value as Ref<UserData>;
  },
}

