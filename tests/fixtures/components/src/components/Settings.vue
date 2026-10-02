<script setup lang="ts">
import { loadUser } from "../user";
const settings = { theme: "dark" };
const user = loadUser();
const rows = [settings];
</script>

<template>
  <li v-for="settings in rows">{{ settings.theme }}</li>
  //                                       ^ d: none
  <li v-for="user in team">{{ user.name }}</li>
  //                               ^ d: picker src/user.ts:2, src/user.ts:10
</template>
