
<script>
export default {
  name: "Counter",
  data() {
    return { count: 0 };
  },
  methods: {
    increment() {
      this.count++;
    },
  },
};
</script>

<template>
  <div>
    <button @click="increment">+1</button>
    <span v-show="count > 0">{{ count }}</span>
    <p v-html="'<b>' + count + '</b>'"></p>
  </div>
</template>
