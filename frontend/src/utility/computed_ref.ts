import { type ComputedGetter, computed, watch, ref } from "vue";

export function computed_ref<T>(callback: ComputedGetter<T>) {
  const _value = computed(callback);
  const value = ref(_value.value);
  watch(_value, (new_value) => value.value = new_value);
  return value;
}
