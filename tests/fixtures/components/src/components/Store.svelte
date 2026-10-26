<script lang="ts">
  import { viewSettings, Converter } from "../config";
  const row = new Converter($viewSettings.sortBy);
  //                                      ^ d: picker src/config.ts:15, src/config.ts:29
</script>
