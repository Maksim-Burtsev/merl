import { writable, derived } from "svelte/store";
import { persisted } from "svelte-persisted-store";
import type { ViewSettings } from "./config";

export const layout = writable<ViewSettings>({ sortBy: "date" });
export const saved = persisted<ViewSettings>("saved", { sortBy: "name" });
export const sorted = derived<ViewSettings>(layout, (v) => v);
