<script setup lang="ts">
import { loadUser } from "../user";
const settings = { theme: "dark" };
const user = loadUser();
const rows = [settings];
const shade = settings.theme;
//                     ^ d: src/components/Settings.vue:3
</script>

<template>
  <li v-for="settings in rows">{{ settings.theme }}</li>
  //                                       ^ d: none
  <li v-for="user in team">{{ user.name }}</li>
  //                               ^ d: picker src/user.ts:2, src/user.ts:10
  <p>{{ user.name }}</p>
  //         ^ d: src/user.ts:2
  <p>{{ settings.theme }}</p>
  //             ^ d: src/components/Settings.vue:3
  <li
    v-for="settings in rows"
    :key="settings.theme"
  >
    {{ settings.theme }}
    //          ^ d: none
  </li>
  <p>{{ settings.theme }}</p>
  //             ^ d: src/components/Settings.vue:3
</template>
