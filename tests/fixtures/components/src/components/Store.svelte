<script lang="ts">
  import { viewSettings, Converter } from "../config";
  import { page } from "$app/stores";
  const row = new Converter($viewSettings.sortBy);
  //                                      ^ d: picker src/config.ts:15, src/config.ts:29
  const path = $page.url;
  //                 ^ d: src/components/Store.svelte:3
  import { layout, saved, sorted } from "../stores";
  import { readable } from "svelte/store";
  import type { ViewSettings } from "../config";
  const local = readable<ViewSettings>({ sortBy: "size" });
  const by = [$layout.sortBy, $saved.sortBy, $local.sortBy];
  //                  ^ d: src/config.ts:29
  //                                 ^ d: src/config.ts:29
  //                                                ^ d: src/config.ts:29
  const kept = $sorted.sortBy;
  //                   ^ d: picker src/config.ts:15, src/config.ts:29
</script>
