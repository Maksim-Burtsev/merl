export {};

declare global {
  interface Array<T> {
    last(): T;
  }
}
