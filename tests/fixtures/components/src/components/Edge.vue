<template>
  <li v-for="item in items">{{ item }}</li>
  <p>{{ item }}</p>
  //    ^ d: picker src/components/Edge.vue:2, src/components/Edge.vue:18
  <Card />
  // ^ d: src/components/UserCard.vue:1
  // status: Card: module src/components/UserCard.vue
  <Lazy />
  // ^ d: src/components/Edge.vue:20
  <Row #default="{ cell: entry }">{{ entry }}</Row>
  //                                  ^ d: src/components/Edge.vue:10
</template>

<script setup lang="ts">
import Card from "./UserCard.vue";
import { nope } from "./UserCard.vue";
//       ^ d: none
const item = 1;
const items = [item];
const Lazy = defineAsyncComponent(() => import("./Legacy.vue"));
</script>

<style>
.box {
  color: red;
  //^ d: none
}
</style>
