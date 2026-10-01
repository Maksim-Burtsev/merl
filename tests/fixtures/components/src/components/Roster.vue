<template>
  <li v-for="user in team">{{ user.name }}</li>
  //                               ^ d: picker src/user.ts:2, src/user.ts:10
  <ul v-for="user in team">
    <li>{{ user.name }}</li>
    //          ^ d: picker src/user.ts:2, src/user.ts:10
  </ul>
  <li v-for="settings in all">{{ settings.theme }}</li>
  //                                       ^ d: none
</template>

<script setup lang="ts">
import { loadUser } from "../user";
const user = loadUser();
const settings = { theme: "dark" };
</script>
