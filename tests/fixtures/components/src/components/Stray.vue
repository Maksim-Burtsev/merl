<template>
  <p v-html="'<!--'">{{ quoted() }}</p>
  //                    ^ d: src/components/Stray.vue:13
  <p :title="`<!--`">{{ ticked() }}</p>
  //                    ^ d: src/components/Stray.vue:14
  <i
    data-x="<!--"
  >{{ "<!--" }}</i>
</template>
<!-->
<!--->
<script setup lang="ts">
function quoted() { return 1 }
function ticked() { return 2 }
quoted()
//^ d: src/components/Stray.vue:13
</script>
