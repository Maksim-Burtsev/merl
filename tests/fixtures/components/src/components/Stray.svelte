<!-- a note
--> <script lang="ts">
  function sameLine() { return 0; }
  sameLine();
  //^ d: src/components/Stray.svelte:3
</script>
<p title="<!--">{sameLine()}</p>
//               ^ d: src/components/Stray.svelte:3
