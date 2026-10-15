
<script lang="ts">
  import { onMount } from "svelte";
  import Card from "./Card.svelte";

  interface Entry {
    id: number;
    name: string;
    amount: number;
  }

  export let title: string | undefined = undefined;
  export let entries: Entry[] = [];

  let query = "";
  let open = true;
  $: total = entries.reduce((sum, e) => sum + e.amount, 0);

  function format(amount: number): string {
    return `${(amount / 100).toFixed(2)} EUR`;
  }

  onMount(async () => {
    const res = await fetch("/api/entries");
    entries = await res.json();
  });
</script>

<!-- a ledger of entries -->
<section class="ledger" class:open>
  {#if title}
    <h1>{title.toUpperCase()}</h1>
  {:else if entries.length}
    <h1>Entries</h1>
  {:else}
    <p>No title</p>
  {/if}

  <ul>
    {#each entries as entry, i (entry.id)}
      <li on:click={() => (open = !open)}>{entry.name}: {format(entry.amount)}</li>
    {/each}
  </ul>

  {#await fetch("/api/total")}
    <p>loading</p>
  {:then res}
    <p>{res.status}</p>
  {:catch error}
    <p>{error.message}</p>
  {/await}

  {#key total}
    <span>{total}</span>
  {/key}

  <input bind:value={query} placeholder="Search" />
  <Card {total} on:close={() => (open = false)} />
  {@html "<b>raw</b>"}
  {@const doubled = total * 2}
  {@debug query}
</section>

<style>
  .ledger {
    display: grid;
    gap: 0.5rem;
    color: #333;
  }

  .ledger li:hover {
    background: rgba(0, 0, 0, 0.05);
  }
</style>
