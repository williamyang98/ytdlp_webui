import { inject, type Ref } from "vue";
import { UserData } from "./user_data.ts";

export const providers = {
  get user_data(): Ref<UserData> {
    const value = inject<Ref<UserData | undefined>>("user_data");
    if (value === undefined) throw Error("Expected user_data to be injected from provider");
    return value as Ref<UserData>;
  },
}
