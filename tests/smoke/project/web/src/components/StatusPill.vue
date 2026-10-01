<script setup lang="ts">
import { ApiClient } from "@/api/client";

const api = new ApiClient("/api");
const label = "paid";
</script>

<template>
  <span class="pill">{{ label }}</span>
</template>

<style scoped>
.pill {
  display: inline-block;
}
</style>
