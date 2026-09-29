// What a namespace declares is in sight of other files; a function's locals are not (#339).
export namespace Tools {
  const stashed = 2;
}

export function outer(): number {
  const stashed = 3;
  return stashed;
}
