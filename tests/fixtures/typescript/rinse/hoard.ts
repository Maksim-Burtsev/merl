export {};

declare global {
  interface Array<T> {
    penult(): T;
  }
}
