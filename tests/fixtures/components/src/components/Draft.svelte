<!--
<script>
  function loadUser() { return 0; }
</script>
-->
<script lang="ts">
  import { loadUser } from "../user";
  loadUser();
  //^ d: src/user.ts:5
</script>
<p>{loadUser()}</p>
//  ^ d: src/user.ts:5
