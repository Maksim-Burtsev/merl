// Members of the types `grading` imports (#341): an enum's, with a value and without, a static
// field's and an exported literal's keys.
export enum Grade {
  Fresh = "fresh",
  Bruised = "bruised",
}

export enum Ripeness {
  GREEN,
  YELLOW,
}

export class Hamper {
  static hamperColors = ["red", "blue"];
  static sizes = [
    SMALL,
  ];
}

export const Pace = {
  Slow: { points: 100 },
  Brisk: { points: 25 },
};

// SMALL is declared nowhere.

export let Tempo = {
  Lazy: 1,
};

export const Stride = Object.freeze({
  Long: 1,
});

export class Manifest {
  static parse(header: string) {
    return header;
  }
}
