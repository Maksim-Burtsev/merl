export const backward = (state: any) => {
  const $cut = findCutBefore(state);
  if (!$cut.nodeBefore) {
    //      ^ d: none
    return false;
  }
  return true;
};

export const forward = (state: any) => {
  for (let d = state.depth; d >= 0; d--) {
    const nodeBefore = state.at(d);
    if (nodeBefore) {
      return d;
    }
  }
  return -1;
};
