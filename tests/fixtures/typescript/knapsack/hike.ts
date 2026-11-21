import { Rucksack } from "./gear";

export function hike({ canteen, thermos }: Rucksack) {
  thermos.quench();
//        ^ d: knapsack/gear.ts:5
  canteen.quench();
//        ^ d: knapsack/gear.ts:2
}

export function roam({ canteen, thermos }) {
  thermos.quench();
//        ^ d: picker knapsack/gear.ts:2, knapsack/gear.ts:5, knapsack/gear.ts:8
}
