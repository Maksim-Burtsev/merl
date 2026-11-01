<script lang="ts"> <!-- f: 1-4 -->
  let items: Array<string> = []; // f: 1-4
  const big = (n: number) => n > 1 && '<b>'; // f: 1-4
</script> <!-- f: 1-4 -->

<button onclick={() => (open = !open)}>toggle</button> <!-- f: none -->
{#if open} <!-- f: 7-17 -->
  <div class="a"> <!-- f: 8-10 -->
    {a < b} <!-- f: 8-10 -->
  </div> <!-- f: 8-10 -->
{:else if other} <!-- f: 11-12 -->
  <p>other</p> <!-- f: 11-12 -->
{:else} <!-- f: 13-16 -->
  <Link
    data-f=" f: 14-16" href="/"
  ><input /></Link>
{/if} <!-- f: 7-17 -->
{#each items as item (item)} <!-- f: 18-22 -->
  <li>{item}</li> <!-- f: 18-22 -->
{:else} <!-- f: 20-21 -->
  <p>none</p> <!-- f: 20-21 -->
{/each} <!-- f: 18-22 -->
{#await load()} <!-- f: 23-29 -->
  <p>loading</p> <!-- f: 23-29 -->
{:then value} <!-- f: 25-26 -->
  <p>{value}</p> <!-- f: 25-26 -->
{:catch error} <!-- f: 27-28 -->
  <p>{error.message}</p> <!-- f: 27-28 -->
{/await} <!-- f: 23-29 -->
{#key id} <!-- f: 30-35 -->
  <img
    data-f=" f: 31-34"
    src="a.png">
  <br> <!-- f: 31-34 -->
{/key} <!-- f: 30-35 -->
{#snippet row(x)} <!-- f: 36-38 -->
  <td>{x}</td> <!-- f: 36-38 -->
{/snippet} <!-- f: 36-38 -->
<svelte:window
  data-f=" f: 39-41"
/>
