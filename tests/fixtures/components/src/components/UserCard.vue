<script setup lang="ts">
import { formatName } from "../names";
const props = defineProps<{ first: string; last: string; tags: string[] }>();
const label = formatName(props.first, props.last);
//            ^ d: src/names.ts:1
//            status: formatName: via import src/names.ts
function save() {}
</script>

<template>
  <div class="card" @click="save">{{ label }}</div>
  //                         ^ d: src/components/UserCard.vue:7
  //                                 ^ d: src/components/UserCard.vue:4
  <span v-for="(tag, i) in props.tags" :key="i">{{ tag }} {{ formatName(tag, "") }}</span>
  //                                                ^ d: src/components/UserCard.vue:14
  // status: tag: local
  //                                                               ^ d: src/names.ts:1
  <user-badge :name="label" />
 //^ d: src/components/UserBadge.svelte:1
  // status: UserBadge: by name, 1 match
  <List v-slot="{ row }">{{ row }}</List>
  //                          ^ d: src/components/UserCard.vue:21
  function shout() {}
  //       ^ d: none
  <div>{{ div }}</div>
  //^ d: none
</template>

<style scoped>
.card {
  display: flex;
  //^ d: none
}
</style>
