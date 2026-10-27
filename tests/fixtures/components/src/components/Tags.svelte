<script lang="ts">
  let label = "All";
  const tags = ["a"];
</script>

<span>{tags.map((label) =>
  label.trim()).join(", ")} {label}</span>
//                           ^ d: src/components/Tags.svelte:2
//^ d: src/components/Tags.svelte:6
