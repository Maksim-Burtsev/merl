import UserCard from "./components/UserCard.vue";
//     ^ d: src/components/UserCard.vue:1
//     status: UserCard: module src/components/UserCard.vue
import Legacy from "@/components/Legacy.vue";
//     ^ d: src/components/Legacy.vue:7
//     status: Legacy: via import src/components/Legacy.vue
import UserBadge from "./components/UserBadge.svelte";
//     ^ d: src/components/UserBadge.svelte:1
import Hero from "./components/Hero.astro";
//     ^ d: src/components/Hero.astro:1

export const root = [UserCard, Legacy, UserBadge, Hero];
//                   ^ d: src/components/UserCard.vue:1
//                                     ^ d: src/components/UserBadge.svelte:1
