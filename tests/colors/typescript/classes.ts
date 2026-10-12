import { Injectable } from "@nestjs/common";

function sealed(constructor: Function) {
  Object.seal(constructor);
}

abstract class Animal {
  protected abstract sound(): string;
  static count = 0;
  #secret = 42;

  constructor(public readonly name: string, private age?: number) {
    Animal.count++;
  }

  get description(): string {
    return `${this.name} says ${this.sound()}`;
  }

  set nickname(value: string) {
    this.name.concat(value);
  }

  override toString(): string {
    return super.toString() + this.#secret;
  }
}

@sealed
@Injectable()
export class Dog extends Animal implements Iterable<number> {
  declare readonly kind: "dog";
  private static instance?: Dog;

  protected sound(): string {
    return "woof";
  }

  public async fetch<T extends object>(url: string): Promise<T> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new TypeError(`bad status ${response.status}`);
    }
    return (await response.json()) as T;
  }

  *[Symbol.iterator](): Iterator<number> {
    yield 1;
    yield* [2, 3];
  }
}

export function run(dogs: Dog[]): void {
  label: for (const dog of dogs) {
    for (const key in dog) {
      if (key === "name") continue label;
    }
    switch (dog.kind) {
      case "dog":
        break;
      default:
        debugger;
    }
  }
  try {
    JSON.parse("{}");
  } catch (e: unknown) {
    console.error(e instanceof Error ? e.message : e);
  } finally {
    dogs.length = 0;
  }
  let i = 0;
  do {
    i += 1;
  } while (i < 10 && !dogs.includes(undefined as never));
  const { name, ...rest } = dogs[0] ?? {};
  const [first, , third = 3] = [1, 2];
  const total = dogs?.map((d) => d.name.length).reduce((a, b) => a + b, 0) ?? NaN;
  delete (rest as any).age;
  void import("./lazy").then((m) => m.default);
  // a line comment
  /* a block comment */
  /** A doc comment with a @param tag. */
  const pattern = /ab+c/gi;
  console.log(name, first, third, total, pattern, Infinity, globalThis, require);
}
