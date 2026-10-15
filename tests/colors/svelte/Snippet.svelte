
<script>
  let { items = [], children } = $props();
  let count = $state(0);
</script>

{#snippet row(item)}
  <tr><td>{item.name}</td><td>{item.size}</td></tr>
{/snippet}

<table>
  {#each items as item}
    {@render row(item)}
  {/each}
</table>

<button onclick={() => count++}>clicked {count} times</button>
{@render children?.()}
