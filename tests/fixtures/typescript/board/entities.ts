export class Entity {
  uid: string;
}

export class ApiKey extends Entity {
  nickname: string;
}

export class Membership extends Entity {
  permission: string;
}

export class Document extends Entity {
  memberships: Membership[];
  insightsEnabled: boolean;
}

export class AuthBoard {
  viewer: ApiKey;
}

export class Board {
  auth: AuthBoard;
}

export function useBoard(): Board {
  return new Board();
}

export class Harness {
  submit(path: string) {
    return path;
  }
}

export function getHarness() {
  const harness = new Harness();
  return harness;
}

export class Team {
  roster: Map<string, Membership>;
  get members() {
    return [new Membership()];
  }
}

export function getMailHarness(urgent: boolean) {
  const harness = new Harness();
  if (urgent) {
    return null;
  }
  return harness;
}

export function getSwappedHarness() {
  let harness = new Harness();
  harness = makeHarness();
  return harness;
}

export function getNullishHarness() {
  let harness = new Harness();
  harness ??= makeHarness();
  return harness;
}

export function getOrHarness() {
  let harness = new Harness();
  harness ||= makeHarness();
  return harness;
}

export function getAndHarness() {
  let harness = new Harness();
  harness &&= makeHarness();
  return harness;
}

export function getArrayHarness() {
  let harness = new Harness();
  [harness] = makeHarnesses();
  return harness;
}

export function getObjectHarness() {
  let harness = new Harness();
  ({ harness } = makeKit());
  return harness;
}
