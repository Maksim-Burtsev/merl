<script lang="ts">
  import { greeting } from "../names";
  export let name: string;
  let count = 0;
  const items = ["a", "b"];
</script>

<button on:click={() => count++}>{greeting} {name}</button>
//                                 ^ d: src/names.ts:5
//                                            ^ d: src/components/UserBadge.svelte:3
{#each items as item, i (item)}
  <span>{item} {i}</span>
  //     ^ d: picker src/components/UserBadge.svelte:11, src/components/UserBadge.svelte:16
  //            ^ d: src/components/UserBadge.svelte:11
{/each}
{#each items as item}
  <b>{item}</b>
{/each}
{#await Promise.resolve(1) then value}
  <i>{value}</i>
  //  ^ d: src/components/UserBadge.svelte:19
{/await}

<style>
  span {
    color: red;
    //^ d: none
  }
</style>
