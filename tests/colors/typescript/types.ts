import { readFile } from "node:fs/promises";
import type { Server } from "http";
import * as path from "path";
import Default, { named as alias } from "./module";

export const MAX_RETRIES = 3;
export const _PRIVATE_LIMIT = 10;
const version: string = "1.0.0";
let counter = 0x1f + 1_000 + 2.5e3 + 10n;

export interface Shape {
  readonly id: number;
  name?: string;
  area(): number;
  [key: string]: unknown;
}

export type Pair<K, V = K> = [K, V];
type Maybe<T> = T | null | undefined;
type Keys = keyof Shape;
type Getters<T> = { [P in keyof T as `get${Capitalize<string & P>}`]: () => T[P] };
type Unwrap<T> = T extends Promise<infer U> ? U : never;
type Fn = (a: number, ...rest: string[]) => void;

enum Color {
  Red = "red",
  Green = "green",
}

declare const enum Flags {
  None = 0,
  All = 1 << 2,
}

declare module "virtual" {
  export function load(id: string): Promise<void>;
}

namespace Geometry {
  export const origin = { x: 0, y: 0 };
}

declare global {
  interface Window {
    merl: boolean;
  }
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function assertDefined<T>(value: T): asserts value is NonNullable<T> {
  if (value === undefined || value === null) {
    throw new Error("undefined");
  }
}

function overload(a: string): string;
function overload(a: number): number;
function overload(a: any): any {
  return a;
}

const config = { port: 8080, host: "localhost" } as const;
const checked = { mode: "dark" } satisfies Record<string, string>;
const forced = <number>(<unknown>"3");
const nonNull = config!.port;
let tuple: readonly [string, number?] = ["a"];
let fnType: new () => object;
let uniq: unique symbol;
let anyValue: any = void 0;
let big: bigint = BigInt(1);

export default function main(args: string[]): number {
  console.log(`args: ${args.length}`, path.sep, Math.max(1, 2));
  return args.length > 0 ? 0 : 1;
}

export { version as appVersion };
export * from "./other";
