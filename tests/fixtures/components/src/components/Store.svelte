<script lang="ts">
  import { viewSettings, Converter } from "../config";
  import { page } from "$app/stores";
  const row = new Converter($viewSettings.sortBy);
  //                                      ^ d: picker src/config.ts:15, src/config.ts:29
  const path = $page.url;
  //                 ^ d: src/components/Store.svelte:3
</script>
