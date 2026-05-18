<script setup lang="ts">
import { ArrowDownWideNarrow, ArrowDownUp, ArrowUpNarrowWide, } from 'lucide-vue-next';

const _props = defineProps<{
  is_descending?: boolean,
}>();
const _emits = defineEmits<{
  click: [],
}>();

</script>

<template>
<ArrowDownUp v-if="is_descending === undefined" class="text-base-content/25 cursor-pointer size-5 select-none" :class="$attrs.class"/>
<ArrowDownWideNarrow v-else-if="is_descending === true" class="cursor-pointer size-5 select-none" :class="$attrs.class"/>
<ArrowUpNarrowWide v-else class="cursor-pointer size-5 select-none" :class="$attrs.class"/>
</template>
