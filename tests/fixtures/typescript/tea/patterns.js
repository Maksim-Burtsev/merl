/** @typedef {import("estree").Pattern} Pattern */

/**
 * @param {Pattern} pattern
 */
function* names(pattern) {
  switch (pattern.type) {
    case "Identifier":
      yield pattern.name;
      //            ^ d: node_modules/@types/estree/index.d.ts:34
      return;
    case "ObjectPattern":
      for (const property of pattern.properties) {
        //                           ^ d: node_modules/@types/estree/index.d.ts:39
        yield* names(property.type === "Property" ? property.value : property);
        //                                                   ^ d: node_modules/@types/estree/index.d.ts:47
        yield* names(property.type !== "Property" ? property : property.value);
        //                                                              ^ d: !jump
      }
      return;
    case "RestElement":
      yield* names(pattern.value);
      //                   ^ d: node_modules/@types/estree/index.d.ts:53
  }
}

/**
 * @param {Pattern} pattern
 */
function fallen(pattern) {
  switch (pattern.type) {
    case "RestElement":
      pattern.argument;
    case "ObjectPattern":
      return pattern.properties;
      //             ^ d: !jump
  }
  switch (pattern.kind) {
    case "ObjectPattern":
      return pattern.properties;
      //             ^ d: !jump
  }
  switch (pattern.type) {
    case "RestElement":
    case "ObjectPattern":
      return pattern.properties;
      //             ^ d: !jump
  }
  switch (pattern.type) {
    case "ObjectPattern":
      [pattern].forEach(() => {
        pattern.properties;
        //      ^ d: !jump
      });
  }
  return pattern.properties;
  //             ^ d: !jump
}
