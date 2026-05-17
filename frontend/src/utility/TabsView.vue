<script lang="ts" setup>
import { useSlots, defineProps, defineExpose, ref, computed } from 'vue';

const props = defineProps<{
  initial_tab?: string;
}>();

interface Tab {
  id: string;
  header_slot?: string;
  body_slot?: string;
}

type SlotType = "header" | "body";

const tabs = computed(() => {
  const slots = useSlots();
  const tabs: Tab[] = [];
  const tabs_table = new Map<string, Tab>();

  for (const slot in slots) {
    let tab_id: string | undefined = undefined;
    let slot_type: SlotType | undefined = undefined;
    if (slot.startsWith("h-")) {
      tab_id = slot.slice(2);
      slot_type = "header";
    } else if (slot.startsWith("b-")) {
      tab_id = slot.slice(2);
      slot_type = "body";
    } else {
      console.warn(`Ignoring slot ${slot} since it isn't a valid h-{...} or b-{...} tab component`);
      continue;
    }

    let tab = tabs_table.get(tab_id);
    if (tab === undefined) {
      tab = { id: tab_id };
      tabs.push(tab);
      tabs_table.set(tab_id, tab);
    }
    switch (slot_type) {
      case "header": {
        if (tab.header_slot !== undefined) {
          console.warn(`Overriding existing header slot ${tab.header_slot} in tab ${tab_id} with slot ${slot}`);
        }
        tab.header_slot = slot;
        break;
      }
      case "body": {
        if (tab.body_slot !== undefined) {
          console.warn(`Overriding existing header slot ${tab.body_slot} in tab ${tab_id} with slot ${slot}`);
        }
        tab.body_slot = slot;
        break;
      }
    }
  }
  return tabs;
});

function get_initial_tab(): string | undefined {
  if (props.initial_tab) return props.initial_tab;
  if (tabs.value.length > 0) return tabs.value[0].id;
  return undefined;
}
const selected_tab = ref(get_initial_tab());

defineExpose({
  set tab(id: string | undefined) {
    selected_tab.value = id;
  },
  get tab(): string | undefined {
    return selected_tab.value;
  },
  get tabs(): string[] {
    return tabs.value.map(tab => tab.id);
  },
})

</script>

<template>
<div class="bg-base-200 p-1 flex flex-col h-full rounded" :class="$attrs.class">
  <div class="w-full flex flex-row gap-x-1 mb-1 max-w-full overflow-x-auto align-middle">
    <template v-for="tab in tabs" :key="tab">
      <button
        class="
          cursor-pointer px-[1rem] py-[0.6rem] text-[0.875rem] rounded
          aria-selected:bg-base-100 aria-selected:shadow
          text-base-content/50
          aria-selected:text-base-content
          hover:text-base-content
          hover:bg-base-100/75
          select-none
        "
        @click="selected_tab = tab.id"
        :aria-selected="selected_tab === tab.id"
      >
        <slot :name="tab.header_slot">{{ tab.id }}</slot>
      </button>
    </template>
  </div>

  <template v-for="tab in tabs" :key="tab">
    <div
      :class="`${selected_tab === tab.id ? '' : 'hidden'}`"
      class="h-full overflow-y-auto"
    >
      <slot :name="tab.body_slot"></slot>
    </div>
  </template>
</div>

</template>
