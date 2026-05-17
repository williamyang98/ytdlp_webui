<script lang="ts" setup>
import { ref, provide } from "vue";
import { UserData } from "./user_data.ts";

const user_data = ref(new UserData(localStorage));
provide("user_data", user_data);
</script>

<template>
<slot></slot>
</template>
