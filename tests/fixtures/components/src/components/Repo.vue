<script setup lang="ts">
import { defaults } from "../config";
const {pageData} = window.config;
const link = pageData.repoLink;
//                    ^ d: picker src/config.ts:2, src/config.ts:9
</script>

<template>
  <li v-for="org in defaults.orgs">{{ org.full_slug }}</li>
  //                                      ^ d: picker src/config.ts:39, src/config.ts:43
</template>
