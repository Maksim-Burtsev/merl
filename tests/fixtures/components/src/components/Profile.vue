<template>
  <p>{{ user.name }}</p>
  //         ^ d: src/user.ts:2
  //         status: name → User.name (via loadUser(): User)
  <p>{{ settings.theme }}</p>
  //             ^ d: src/components/Profile.vue:15
</template>

<script setup lang="ts">
import { loadUser } from "../user";
//       ^ d: src/user.ts:5
const user = loadUser();
//    ^ d: src/components/Profile.vue:12
const settings = {
  theme: "dark",
};
</script>
