export class Canteen {
  quench(): void {}
}
export class Thermos {
  quench(): void {}
}
export class Jerrycan {
  quench(): void {}
}
export interface Rucksack { canteen: Canteen; thermos: Thermos }
