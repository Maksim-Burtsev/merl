// What the project declares into the global scope is a member of `window` and `globalThis` (#341).
export {};

declare global {
  interface Window {
    dataLayer: unknown[];
  }
  var appConfig: { name: string };
}
