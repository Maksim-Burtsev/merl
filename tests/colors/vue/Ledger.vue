
<template>
  <!-- a ledger of entries -->
  <section class="ledger" :class="{ open: isOpen }">
    <h1 v-if="title">{{ title.toUpperCase() }}</h1>
    <p v-else>No title</p>
    <ul>
      <li v-for="(entry, i) in entries" :key="entry.id" @click.stop="pick(i)">
        {{ entry.name }}: {{ format(entry.amount) }}
      </li>
    </ul>
    <input v-model.trim="query" type="text" placeholder="Search" />
    <Card v-bind:total="total" v-on:close="isOpen = false">
      <template #footer="{ count }">
        <span>{{ count }} items</span>
      </template>
    </Card>
    <button :[attr]="value" :disabled="!entries.length">Add</button>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import Card from "./Card.vue";

interface Entry {
  id: number;
  name: string;
  amount: number;
}

const props = defineProps<{ title?: string; entries: Entry[] }>();
const emit = defineEmits<{ (e: "pick", index: number): void }>();

const isOpen = ref(true);
const query = ref("");
const attr = "data-kind";
const value = 42;

const total = computed(() =>
  props.entries.reduce((sum, e) => sum + e.amount, 0),
);

function format(amount: number): string {
  return `${(amount / 100).toFixed(2)} EUR`;
}

function pick(index: number) {
  emit("pick", index);
}
</script>

<style scoped>
.ledger {
  display: grid;
  gap: 0.5rem;
  color: #333;
}

.ledger li:hover {
  background: rgba(0, 0, 0, 0.05);
}
</style>
