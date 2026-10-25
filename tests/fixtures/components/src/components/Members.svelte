<script lang="ts">
  import { loadUser } from "../user";
  const user = loadUser();
  const settings = { theme: "dark" };
  const team = [user];
</script>

{#each team as user}
  <span>{user.name}</span>
  //          ^ d: picker src/user.ts:2, src/user.ts:10
{/each}
{#each [settings] as settings}
  <span>{settings.theme}</span>
  //              ^ d: none
{/each}
<span>{settings.theme}</span>
//              ^ d: src/components/Members.svelte:4
<span>{user.name}</span>
//          ^ d: picker src/user.ts:2, src/user.ts:10
{#each team as member (member.name)}
//             ^ d: src/components/Members.svelte:20
  <span>{member.name}</span>
{/each}
{#each team as member (member.name)}
  <span>{member.name}</span>
  //     ^ d: src/components/Members.svelte:24
{/each}
