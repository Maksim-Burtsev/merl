<template> <!-- f: 1-21 -->
  <div class="a" @click="n => n > 0 && go(n)"> <!-- f: 2-15 -->
    <input
      data-f=" f: 3-6"
      type="text"
    >
    <button
      data-f=" f: 7-9"
    >{{ a < b }}</button>
    <input data-f=" f: 10-11">
    <span>after the input</span> <!-- f: 10-11 -->
    <MyIcon
      data-f=" f: 12-14"
    />
  </div> <!-- f: 2-15 -->
  <template v-if="ok"> <!-- f: 16-18 -->
    <p>one</p> <!-- f: 16-18 -->
  </template> <!-- f: 16-18 -->
  <input data-f=" f: 19-20">
  <input data-f=" f: 19-20">
</template> <!-- f: 1-21 -->

<script setup lang="ts"> <!-- f: 23-28 -->
const tags: Array<string> = ['</template>'] // f: 23-28
function go(n: number) { // f: 23-28
  return n < 1 ? '<div>' : n
}
</script> <!-- f: 23-28 -->

<style scoped> <!-- f: 30-32 -->
.a > .b { color: red } /* f: 30-32 */
</style>
<!-- a comment f: none -->
