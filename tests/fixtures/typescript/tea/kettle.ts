export default class Kettle {
  heat = 90;

  boil(): number;
  boil(degrees: number): number;
  boil(degrees?: number): number {
    return degrees ?? this.heat;
  }
}

export function steep(minutes: number): number {
  return minutes;
}

export namespace steep {
  export const minutes = 3;
}
