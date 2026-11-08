<template>
  <!-- <script>function loadUser() {}</script> -->
  <p>{{ loadUser() }}</p>
  //     ^ d: src/user.ts:5
</template>
<!--
<script setup lang="ts">
function loadUser() { return 0; }
class Team {}
function draftOnly() {}
</script>
-->
<script setup lang="ts">
import { loadUser, Team } from "../user";
loadUser();
//^ d: src/user.ts:5
new Team();
//  ^ d: src/user.ts:9
draftOnly();
//^ d: none
</script>
